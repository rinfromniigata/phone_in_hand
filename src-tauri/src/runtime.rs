//! 映像のスレッドと姿勢のスレッド（T3-6。imple 2章「映像の経路」「姿勢の経路」）。
//!
//! - 映像のスレッド：`video_source.next_frame` → `frame_processor.analyze` → `overlay_presenter.set_frame`
//!   と直接渡す。GPUのフレームはCPUを経由しない。UIのプレビューには、縮小画像をJPEGにして送る（5fps程度）
//! - 姿勢のスレッド：SteamVRのフレームに合わせて回り、`pose_source.sample` と `pose_fusion.update` の結果を
//!   `overlay_presenter.set_state` に渡す。SteamVRとの接続と切断もこのスレッドが受け持つ
//! - 状態の変化は [`RuntimeEvent`] で `app_state` へ知らせる。表示（presenter）のロックを持ったまま
//!   知らせない（`app_state` のロックとの順序の食い違いを避ける）
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use base64::Engine;
use pih_core::calibration::{CalibrationInput, CalibrationSession, CalibrationStep};
use pih_core::frame::{BlackBarProcessor, FrameInfo, FrameProcessor};
use pih_core::geometry::{Pose, Rect, Size};
use pih_core::overlay::OverlayPresenter;
use pih_core::pose::{PanelState, PanelVisibility, PoseFusion, PoseSource, effective_offset};
use pih_core::settings::{Calibration, CropSettings, PoseSettings, Settings, VideoSettings};
use pih_core::snapshot::TrackingStatus;
use pih_core::video::{
    Frame, Thumbnail, VideoConfig, VideoSource, VideoSourceKind, VideoStatus, WindowInfo,
};
use pih_core::{Error, Result};
use serde::Serialize;

use crate::controller_pose::ControllerPoseSource;
use crate::gpu::Gpu;
use crate::openvr::{OpenVr, VrError};
use crate::overlay::{OpenVrOverlayPresenter, OverlayImages, shown_at};
use crate::startup;
use crate::window_capture::{self, WindowCaptureSource};

/// 取り込みに失敗したとき、やり直すまでの間隔（ウィンドウを開き直したら戻る）
const VIDEO_RETRY: Duration = Duration::from_secs(2);
/// SteamVRにつながらないとき、つなぎ直すまでの間隔
const VR_RETRY: Duration = Duration::from_secs(3);
/// SteamVRが入っていないとき、つなぎ直すまでの間隔
const VR_RETRY_NOT_INSTALLED: Duration = Duration::from_secs(15);
/// SteamVRのフレームを待つ上限
const FRAME_SYNC_TIMEOUT_MS: u32 = 100;
/// プレビューのJPEGの品質
const PREVIEW_JPEG_QUALITY: u8 = 70;
/// 画面が変わらずフレームが来ない間、最後のプレビューを送り直す間隔
const PREVIEW_RESEND: Duration = Duration::from_secs(1);

/// 各スレッドから `app_state` への知らせ
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Video(VideoStatus),
    Tracking(TrackingStatus),
    /// SteamVRとつながった・切れた
    SteamVr(bool),
    /// 位置合わせをVRの中で決定した
    CalibrationFinished(Calibration),
    /// 位置合わせの途中でSteamVRとの接続が切れた
    CalibrationFailed,
}

/// UIのプレビュー（`preview_frame` イベント）。切り抜く前の縮小画像と、元の大きさでの切り抜きの範囲
#[derive(Debug, Clone, Serialize)]
pub struct PreviewFrame {
    /// JPEGをbase64にしたもの
    pub jpeg: String,
    pub width: u32,
    pub height: u32,
    pub frame: Size,
    pub crop: Rect,
}

pub type EventSink = Arc<dyn Fn(RuntimeEvent) + Send + Sync>;
pub type PreviewSink = Arc<dyn Fn(PreviewFrame) + Send + Sync>;

/// `app_state` から各スレッドへの操作（テストでは偽の実装に差し替える）
pub trait RuntimeLink: Send + Sync {
    /// 設定・位置合わせの結果が変わった
    fn configure(&self, settings: &Settings, calibration: Option<&Calibration>);
    /// 保存していない映像の設定で取り込む（映像のページのプレビュー）
    fn preview_video(&self, video: &VideoSettings);
    /// UIが描いたチップと見本の画像
    fn set_overlay_images(&self, images: OverlayImages);
    fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    /// 位置合わせを始める。`start_offset` は見本を最初に出すずれ。SteamVRとつながっていなければ失敗する
    fn start_calibration(&self, pose: PoseSettings, start_offset: Pose) -> Result<()>;
    /// 位置合わせをやめる（結果は保存しない）
    fn cancel_calibration(&self);
    /// 位置合わせを、今の見本の位置で決定する（PCから決定するとき）
    fn finish_calibration(&self) -> Result<Calibration>;
    /// SteamVRと一緒に起動する設定を、SteamVRへの登録に合わせる（つながっていなければ何もしない）
    fn sync_startup(&self, start_with_steamvr: bool);
}

impl<T: RuntimeLink> RuntimeLink for Arc<T> {
    fn sync_startup(&self, start_with_steamvr: bool) {
        (**self).sync_startup(start_with_steamvr)
    }
    fn start_calibration(&self, pose: PoseSettings, start_offset: Pose) -> Result<()> {
        (**self).start_calibration(pose, start_offset)
    }
    fn cancel_calibration(&self) {
        (**self).cancel_calibration()
    }
    fn finish_calibration(&self) -> Result<Calibration> {
        (**self).finish_calibration()
    }
    fn configure(&self, settings: &Settings, calibration: Option<&Calibration>) {
        (**self).configure(settings, calibration)
    }
    fn preview_video(&self, video: &VideoSettings) {
        (**self).preview_video(video)
    }
    fn set_overlay_images(&self, images: OverlayImages) {
        (**self).set_overlay_images(images)
    }
    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        (**self).list_windows()
    }
}

/// 姿勢のスレッドが毎フレーム使う設定
#[derive(Debug, Clone, Copy, PartialEq)]
struct PoseConfig {
    pose: PoseSettings,
    offset: Pose,
}

struct Shared {
    vr: Arc<OpenVr>,
    presenter: Option<Mutex<OpenVrOverlayPresenter>>,
    pose_config: Mutex<PoseConfig>,
    /// 位置合わせの途中なら、その手順
    calibration: Mutex<Option<CalibrationSession>>,
    /// 位置合わせの入力の定義（actions.json）の場所
    input_manifest: PathBuf,
    screen_off: AtomicBool,
    quit: AtomicBool,
    events: EventSink,
}

impl Shared {
    fn calibration(&self) -> MutexGuard<'_, Option<CalibrationSession>> {
        self.calibration
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// 位置合わせを終える（表示を通常に戻す）
    fn end_calibration(&self) -> Option<CalibrationSession> {
        let session = self.calibration().take();
        if session.is_some()
            && let Some(mut p) = self.presenter()
        {
            p.set_calibrating(false);
        }
        session
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn calibration_result(pose: PoseSettings, offset: Pose) -> Calibration {
    Calibration {
        source: pose.source,
        side: pose.side,
        offset,
        calibrated_at_ms: now_ms(),
    }
}

impl Shared {
    fn presenter(&self) -> Option<MutexGuard<'_, OpenVrOverlayPresenter>> {
        self.presenter
            .as_ref()
            .map(|p| p.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

enum VideoCmd {
    Configure(VideoSettings),
}

pub struct Runtime {
    shared: Arc<Shared>,
    video_tx: Mutex<Sender<VideoCmd>>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    /// SteamVRに登録するアプリのマニフェスト（書けなかったら `None`）
    app_manifest: Mutex<Option<PathBuf>>,
}

impl Runtime {
    /// スレッドを始める。GPUを使えない環境では、映像と表示は「使用できません」のまま
    pub fn start(
        vr: Arc<OpenVr>,
        events: EventSink,
        preview: PreviewSink,
        settings: &Settings,
        calibration: Option<&Calibration>,
        input_manifest: PathBuf,
    ) -> Runtime {
        let gpu = match Gpu::new() {
            Ok(g) => Some(g),
            Err(e) => {
                log::warn!("D3D11のデバイスを作れません: {e}");
                None
            }
        };
        let presenter = gpu
            .clone()
            .map(|g| Mutex::new(OpenVrOverlayPresenter::new(vr.clone(), g)));
        let shared = Arc::new(Shared {
            vr,
            presenter,
            pose_config: Mutex::new(PoseConfig {
                pose: settings.pose,
                offset: effective_offset(calibration, &settings.pose),
            }),
            calibration: Mutex::new(None),
            input_manifest,
            screen_off: AtomicBool::new(true),
            quit: AtomicBool::new(false),
            events,
        });
        let (video_tx, video_rx) = mpsc::channel();
        let mut threads = Vec::new();
        {
            let shared = shared.clone();
            threads.push(
                std::thread::Builder::new()
                    .name("pih-video".into())
                    .spawn(move || video_loop(shared, gpu, video_rx, preview))
                    .expect("映像のスレッドを始められません"),
            );
        }
        {
            let shared = shared.clone();
            threads.push(
                std::thread::Builder::new()
                    .name("pih-pose".into())
                    .spawn(move || pose_loop(shared))
                    .expect("姿勢のスレッドを始められません"),
            );
        }
        let rt = Runtime {
            shared,
            video_tx: Mutex::new(video_tx),
            threads: Mutex::new(threads),
            app_manifest: Mutex::new(None),
        };
        rt.configure(settings, calibration);
        rt
    }

    /// SteamVRと一緒に起動するためのマニフェストの場所を設定する
    pub fn set_app_manifest(&self, path: PathBuf) {
        *self
            .app_manifest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(path);
    }

    fn send_video(&self, cmd: VideoCmd) {
        let tx = self
            .video_tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if tx.send(cmd).is_err() {
            log::warn!("映像のスレッドが止まっています");
        }
    }

    /// スレッドを止める（アプリの終了）
    pub fn shutdown(&self) {
        self.shared.quit.store(true, Ordering::SeqCst);
        let threads = std::mem::take(
            &mut *self
                .threads
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        for t in threads {
            let _ = t.join();
        }
        self.shared.vr.disconnect();
    }
}

impl RuntimeLink for Runtime {
    fn configure(&self, settings: &Settings, calibration: Option<&Calibration>) {
        {
            let mut c = self
                .shared
                .pose_config
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *c = PoseConfig {
                pose: settings.pose,
                offset: effective_offset(calibration, &settings.pose),
            };
        }
        if let Some(mut p) = self.shared.presenter() {
            let d = settings.display;
            let _ = p.set_appearance(d.size_inch, d.brightness);
            let _ = p.set_enabled(d.enabled);
        }
        self.send_video(VideoCmd::Configure(settings.video.clone()));
    }

    fn preview_video(&self, video: &VideoSettings) {
        self.send_video(VideoCmd::Configure(video.clone()));
    }

    fn set_overlay_images(&self, images: OverlayImages) {
        if let Some(mut p) = self.shared.presenter() {
            p.set_images(images);
        }
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        window_capture::list_windows()
    }

    fn start_calibration(&self, pose: PoseSettings, start_offset: Pose) -> Result<()> {
        if self.shared.vr.generation().is_none() || self.shared.presenter.is_none() {
            return Err(Error::Unavailable);
        }
        *self.shared.calibration() = Some(CalibrationSession::new(pose, start_offset));
        if let Some(mut p) = self.shared.presenter() {
            p.set_calibrating(true);
        }
        Ok(())
    }

    fn cancel_calibration(&self) {
        self.shared.end_calibration();
    }

    fn sync_startup(&self, start_with_steamvr: bool) {
        let manifest = self
            .app_manifest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(manifest) = manifest else {
            return;
        };
        match startup::sync(&self.shared.vr, &manifest, start_with_steamvr) {
            Ok(()) => log::info!("SteamVRと一緒に起動する設定を反映しました: {start_with_steamvr}"),
            Err(VrError::NotConnected) => {} // 次につながったときに合わせる
            Err(e) => log::warn!("SteamVRと一緒に起動する設定を反映できません: {e}"),
        }
    }

    fn finish_calibration(&self) -> Result<Calibration> {
        let session = self.shared.end_calibration().ok_or(Error::Unavailable)?;
        Ok(calibration_result(
            session.pose_settings(),
            session.offset(),
        ))
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ---- 映像のスレッド ----

/// 取り込み方法の設定から、取り込みの開始に使う設定を作る。まだ選んでいなければ `None`
fn video_config(video: &VideoSettings) -> Option<VideoConfig> {
    match video.source {
        VideoSourceKind::Window => {
            let w = &video.window;
            let title = w.title.clone().unwrap_or_default();
            let executable = w.executable.clone().unwrap_or_default();
            (!title.is_empty() || !executable.is_empty())
                .then_some(VideoConfig::Window { title, executable })
        }
        // HDMIキャプチャはPhase 4、Androidのミラーリングは Phase 7 でつなぐ
        VideoSourceKind::Hdmi | VideoSourceKind::Mirror => None,
    }
}

struct VideoState {
    gpu: Option<Arc<Gpu>>,
    source: Option<Box<dyn VideoSource>>,
    config: Option<VideoConfig>,
    kind: Option<VideoSourceKind>,
    crop: CropSettings,
    processor: BlackBarProcessor,
    status: VideoStatus,
    last_info: Option<FrameInfo>,
    retry_at: Option<Instant>,
    /// 最後に送ったプレビューと、その時刻
    last_preview: Option<(PreviewFrame, Instant)>,
}

impl VideoState {
    fn apply(&mut self, shared: &Shared, video: VideoSettings) {
        if video.crop != self.crop {
            self.crop = video.crop;
            self.processor.set_crop(video.crop);
        }
        let config = video_config(&video);
        if Some(video.source) == self.kind && config == self.config {
            return;
        }
        self.kind = Some(video.source);
        self.config = config;
        self.restart(shared);
    }

    fn restart(&mut self, shared: &Shared) {
        if let Some(mut s) = self.source.take() {
            s.stop();
        }
        self.last_info = None;
        self.retry_at = None;
        self.last_preview = None;
        if let Some(mut p) = shared.presenter() {
            p.clear_frame();
        }
        let Some(config) = self.config.clone() else {
            self.status = match self.kind {
                Some(VideoSourceKind::Window) => VideoStatus::Idle,
                _ => VideoStatus::Unavailable,
            };
            return;
        };
        let Some(gpu) = self.gpu.clone() else {
            self.status = VideoStatus::Unavailable;
            return;
        };
        let mut source: Box<dyn VideoSource> = match config {
            VideoConfig::Window { .. } => Box::new(WindowCaptureSource::new(gpu)),
            _ => {
                self.status = VideoStatus::Unavailable;
                return;
            }
        };
        if let Err(e) = source.start(&config) {
            log::info!("取り込みを始められません: {e}");
            self.retry_at = Some(Instant::now() + VIDEO_RETRY);
        }
        self.status = source.status();
        self.source = Some(source);
    }

    /// 1回分：フレームを待ち、判定して表示へ渡す
    fn step(&mut self, shared: &Shared, preview: &PreviewSink, wait: Duration) {
        let Some(source) = self.source.as_mut() else {
            return;
        };
        if let Some(at) = self.retry_at {
            if Instant::now() >= at {
                self.retry_at = None;
                if let Some(config) = self.config.clone()
                    && source.start(&config).is_err()
                {
                    self.retry_at = Some(Instant::now() + VIDEO_RETRY);
                }
            } else {
                std::thread::sleep(wait.min(at - Instant::now()));
            }
            self.status = source.status();
            return;
        }
        match source.next_frame(wait) {
            Ok(Some(frame)) => {
                let now = Instant::now();
                let info = self.processor.analyze(&frame, now);
                self.last_info = Some(info);
                if let Some(mut p) = shared.presenter()
                    && let Err(e) = p.set_frame(&frame, info.crop)
                {
                    log::debug!("フレームを表示へ渡せません: {e}");
                }
                if let Frame::Gpu {
                    thumbnail: Some(t),
                    size,
                    ..
                } = &frame
                    && let Some(p) = encode_preview(t, *size, info.crop)
                {
                    self.last_preview = Some((p.clone(), now));
                    preview(p);
                }
            }
            Ok(None) => {
                // 画面が変わらずフレームが来ない間も、黒い画面の判定の時間を進め、切り抜きの変更を表示へ渡す
                let now = Instant::now();
                if let Some(info) = self.processor.tick(now) {
                    if self.last_info.map(|i| i.crop) != Some(info.crop)
                        && let Some(mut p) = shared.presenter()
                    {
                        p.set_crop(info.crop);
                    }
                    self.last_info = Some(info);
                }
                // 最後のプレビューを送り直す（あとから開いた画面にも出るように）
                if let Some((p, at)) = self.last_preview.as_mut()
                    && now.duration_since(*at) >= PREVIEW_RESEND
                {
                    *at = now;
                    if let Some(info) = self.last_info {
                        p.crop = info.crop;
                    }
                    preview(p.clone());
                }
            }
            Err(e) => {
                log::debug!("フレームを受け取れません: {e}");
                std::thread::sleep(wait);
            }
        }
        self.status = source.status();
        if matches!(self.status, VideoStatus::Error { .. }) {
            if let Some(mut p) = shared.presenter() {
                p.clear_frame();
            }
            self.last_info = None;
            self.last_preview = None;
            self.retry_at = Some(Instant::now() + VIDEO_RETRY);
        }
    }
}

fn encode_preview(t: &Thumbnail, frame: Size, crop: Rect) -> Option<PreviewFrame> {
    let (w, h) = (
        u16::try_from(t.size.width).ok()?,
        u16::try_from(t.size.height).ok()?,
    );
    let mut out = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut out, PREVIEW_JPEG_QUALITY);
    if let Err(e) = encoder.encode(&t.bgra, w, h, jpeg_encoder::ColorType::Bgra) {
        log::debug!("プレビューを作れません: {e}");
        return None;
    }
    Some(PreviewFrame {
        jpeg: base64::engine::general_purpose::STANDARD.encode(&out),
        width: t.size.width,
        height: t.size.height,
        frame,
        crop,
    })
}

fn video_loop(
    shared: Arc<Shared>,
    gpu: Option<Arc<Gpu>>,
    rx: Receiver<VideoCmd>,
    preview: PreviewSink,
) {
    let mut st = VideoState {
        gpu,
        source: None,
        config: None,
        kind: None,
        crop: CropSettings::default(),
        processor: BlackBarProcessor::new(CropSettings::default()),
        status: VideoStatus::Unavailable,
        last_info: None,
        retry_at: None,
        last_preview: None,
    };
    let mut reported: Option<VideoStatus> = None;
    const WAIT: Duration = Duration::from_millis(100);
    while !shared.quit.load(Ordering::SeqCst) {
        // 取り込んでいない間は、設定の変更を待つ
        let cmd = if st.source.is_some() {
            rx.try_recv().map_err(|e| match e {
                mpsc::TryRecvError::Empty => RecvTimeoutError::Timeout,
                mpsc::TryRecvError::Disconnected => RecvTimeoutError::Disconnected,
            })
        } else {
            rx.recv_timeout(WAIT)
        };
        match cmd {
            Ok(VideoCmd::Configure(v)) => {
                // 溜まっている設定は、いちばん新しいものだけを使う
                let mut latest = v;
                while let Ok(VideoCmd::Configure(v)) = rx.try_recv() {
                    latest = v;
                }
                st.apply(&shared, latest);
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        st.step(&shared, &preview, WAIT);

        let off = pih_core::frame::screen_off(&st.status, st.last_info.as_ref());
        shared.screen_off.store(off, Ordering::SeqCst);
        if reported.as_ref() != Some(&st.status) {
            reported = Some(st.status.clone());
            (shared.events)(RuntimeEvent::Video(st.status.clone()));
        }
    }
    if let Some(mut s) = st.source.take() {
        s.stop();
    }
}

// ---- 姿勢のスレッド ----

fn tracking_status(
    state: &pih_core::pose::PanelState,
    tracked: bool,
    pose: &PoseSettings,
) -> TrackingStatus {
    if tracked {
        TrackingStatus::Tracking {
            mode: state.mode,
            side: pose.side,
        }
    } else {
        TrackingStatus::Lost { side: pose.side }
    }
}

/// 位置合わせの途中なら1フレーム分を進めて `true` を返す。見本はスマホを付けた側のコントローラーに付けて出し、
/// もう一方のコントローラーのグリップでつかんで動かし、どちらかのトリガーで決定する
fn calibration_step(shared: &Shared, source: &mut ControllerPoseSource) -> bool {
    let Some(pose) = shared.calibration().as_ref().map(|s| s.pose_settings()) else {
        return false;
    };
    let side = pose.side;
    let other_side = side.other();
    let other_buttons = shared.vr.poll_buttons(other_side).unwrap_or_default();
    let phone_buttons = shared.vr.poll_buttons(side).unwrap_or_default();
    let input = CalibrationInput {
        phone: source.sample(side).tracked(),
        other: source.sample(other_side).tracked(),
        grab: other_buttons.grab,
        confirm: other_buttons.confirm || phone_buttons.confirm,
    };
    let step = match shared.calibration().as_mut() {
        Some(session) => session.update(&input),
        None => return false, // ちょうどやめた
    };
    match step {
        CalibrationStep::Continue { sample } => {
            if let Some(mut p) = shared.presenter() {
                let state = match sample {
                    Some(pose) => shown_at(pose),
                    None => PanelState {
                        has_pose: false,
                        visibility: PanelVisibility::Hidden,
                        ..shown_at(Pose::default())
                    },
                };
                let _ = p.set_state(&state);
            }
        }
        CalibrationStep::Finished { offset } => {
            if shared.end_calibration().is_some() {
                (shared.events)(RuntimeEvent::CalibrationFinished(calibration_result(
                    pose, offset,
                )));
            }
        }
    }
    true
}

fn pose_loop(shared: Arc<Shared>) {
    let vr = shared.vr.clone();
    let mut source = ControllerPoseSource::new(vr.clone());
    let mut fusion = PoseFusion::new();
    let mut reported: Option<TrackingStatus> = None;
    let mut next_connect = Instant::now();
    let mut last_pose_settings: Option<PoseSettings> = None;
    let report = |reported: &mut Option<TrackingStatus>, s: TrackingStatus| {
        if reported.as_ref() != Some(&s) {
            *reported = Some(s.clone());
            (shared.events)(RuntimeEvent::Tracking(s));
        }
    };

    while !shared.quit.load(Ordering::SeqCst) {
        if vr.generation().is_none() {
            report(&mut reported, TrackingStatus::Unavailable);
            let now = Instant::now();
            if now >= next_connect {
                match vr.connect() {
                    Ok(_) => {
                        log::info!("SteamVRにつながりました");
                        if let Err(e) = vr.init_input(&shared.input_manifest) {
                            log::warn!("位置合わせの入力を使えません: {e}");
                        }
                        fusion.reset();
                        (shared.events)(RuntimeEvent::SteamVr(true));
                        continue;
                    }
                    Err(e) => {
                        log::debug!("{e}");
                        next_connect = now
                            + if matches!(e, VrError::NotLoaded) || !vr.runtime_installed() {
                                VR_RETRY_NOT_INSTALLED
                            } else {
                                VR_RETRY
                            };
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }

        if vr.wait_frame_sync(FRAME_SYNC_TIMEOUT_MS).is_err() {
            std::thread::sleep(Duration::from_millis(11));
        }
        if vr.poll_quit() {
            log::info!("SteamVRが終了しました");
            if let Some(mut p) = shared.presenter() {
                p.on_disconnected();
            }
            fusion.reset();
            next_connect = Instant::now() + VR_RETRY;
            if shared.end_calibration().is_some() {
                (shared.events)(RuntimeEvent::CalibrationFailed);
            }
            (shared.events)(RuntimeEvent::SteamVr(false));
            continue;
        }

        if calibration_step(&shared, &mut source) {
            continue;
        }

        let cfg = *shared
            .pose_config
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if last_pose_settings != Some(cfg.pose) {
            // 基準や側を変えたら、前の側の最後の姿勢は使わない
            fusion.reset();
            last_pose_settings = Some(cfg.pose);
        }
        let sample = source.sample(cfg.pose.side);
        let calibration = Calibration {
            source: cfg.pose.source,
            side: cfg.pose.side,
            offset: cfg.offset,
            calibrated_at_ms: 0,
        };
        let screen_off = shared.screen_off.load(Ordering::SeqCst);
        let state = fusion.update(&sample, None, &calibration, screen_off, Instant::now());
        if let Some(mut p) = shared.presenter() {
            let _ = p.set_state(&state);
        }
        report(
            &mut reported,
            tracking_status(&state, sample.tracked().is_some(), &cfg.pose),
        );
    }
}

/// テスト用の何もしない実装（受け取った設定を記録する）
#[cfg(test)]
#[derive(Default)]
pub struct NullRuntime {
    pub configured: Mutex<Vec<Settings>>,
    pub previews: Mutex<Vec<VideoSettings>>,
    /// 位置合わせの途中なら、その基準と最初のずれ
    pub calibrating: Mutex<Option<(PoseSettings, Pose)>>,
    /// SteamVRと一緒に起動する設定を合わせた記録
    pub startup: Mutex<Vec<bool>>,
}

#[cfg(test)]
impl RuntimeLink for NullRuntime {
    fn configure(&self, settings: &Settings, _calibration: Option<&Calibration>) {
        self.configured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(settings.clone());
    }
    fn preview_video(&self, video: &VideoSettings) {
        self.previews
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(video.clone());
    }
    fn set_overlay_images(&self, _images: OverlayImages) {}
    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(Error::Unavailable)
    }
    fn start_calibration(&self, pose: PoseSettings, start_offset: Pose) -> Result<()> {
        *self
            .calibrating
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((pose, start_offset));
        Ok(())
    }
    fn cancel_calibration(&self) {
        self.calibrating
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
    fn finish_calibration(&self) -> Result<Calibration> {
        let (pose, offset) = self
            .calibrating
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .ok_or(Error::Unavailable)?;
        Ok(calibration_result(pose, offset))
    }
    fn sync_startup(&self, start_with_steamvr: bool) {
        self.startup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(start_with_steamvr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pih_core::settings::WindowSettings;

    #[test]
    fn video_config_needs_a_chosen_window() {
        let mut v = VideoSettings {
            source: VideoSourceKind::Window,
            ..VideoSettings::default()
        };
        assert_eq!(video_config(&v), None);
        v.window = WindowSettings {
            title: Some("AirPlay".into()),
            executable: Some("air.exe".into()),
        };
        assert_eq!(
            video_config(&v),
            Some(VideoConfig::Window {
                title: "AirPlay".into(),
                executable: "air.exe".into()
            })
        );
        v.source = VideoSourceKind::Hdmi;
        assert_eq!(video_config(&v), None);
    }

    #[test]
    fn preview_is_jpeg() {
        let t = Thumbnail {
            size: Size {
                width: 4,
                height: 2,
            },
            bgra: vec![128; 4 * 2 * 4],
        };
        let p = encode_preview(
            &t,
            Size {
                width: 40,
                height: 20,
            },
            Rect::default(),
        )
        .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&p.jpeg)
            .unwrap();
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
    }

    /// デスクトップの実際のウィンドウを取り込み、状態とプレビューが届く（手元での確認用。`--ignored`）
    #[test]
    #[ignore]
    fn runs_with_a_real_window() {
        let list = window_capture::list_windows().unwrap();
        let w = list.first().expect("取り込めるウィンドウがない");
        let events = Arc::new(Mutex::new(Vec::new()));
        let previews = Arc::new(Mutex::new(Vec::new()));
        let (e, p) = (events.clone(), previews.clone());
        let mut settings = Settings::default();
        settings.video.source = VideoSourceKind::Window;
        settings.video.window = WindowSettings {
            title: Some(w.title.clone()),
            executable: Some(w.executable.clone()),
        };
        let rt = Runtime::start(
            Arc::new(OpenVr::new("Z:/no/openvr_api.dll".into())),
            Arc::new(move |ev| e.lock().unwrap().push(ev)),
            Arc::new(move |f| p.lock().unwrap().push(f)),
            &settings,
            None,
            PathBuf::new(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while previews.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "プレビューが届かない");
            std::thread::sleep(Duration::from_millis(50));
        }
        rt.shutdown();
        assert!(
            events
                .lock()
                .unwrap()
                .iter()
                .any(|e| matches!(e, RuntimeEvent::Video(VideoStatus::Running { .. })))
        );
        let f = previews.lock().unwrap()[0].clone();
        assert!(f.width > 0 && f.crop.width > 0 && f.frame.width >= f.width);
    }

    /// SteamVRもウィンドウもない状態で始めて止められる。映像は「待機中」を、追跡は「使用できません」を知らせる
    #[test]
    fn starts_and_stops_without_devices() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let e = events.clone();
        let sink: EventSink = Arc::new(move |ev| e.lock().unwrap().push(ev));
        let vr = Arc::new(OpenVr::new("Z:/no/openvr_api.dll".into()));
        let mut settings = Settings::default();
        settings.video.source = VideoSourceKind::Window;
        let rt = Runtime::start(vr, sink, Arc::new(|_| {}), &settings, None, PathBuf::new());
        // SteamVRとつながっていなければ、位置合わせは始められない
        assert!(matches!(
            rt.start_calibration(settings.pose, Pose::default()),
            Err(Error::Unavailable)
        ));
        assert!(rt.finish_calibration().is_err());
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let got = events.lock().unwrap().clone();
            let video = got.iter().any(|e| matches!(e, RuntimeEvent::Video(_)));
            let tracking = got.contains(&RuntimeEvent::Tracking(TrackingStatus::Unavailable));
            if video && tracking {
                break;
            }
            assert!(Instant::now() < deadline, "知らせが届かない: {got:?}");
            std::thread::sleep(Duration::from_millis(20));
        }
        rt.shutdown();
        let got = events.lock().unwrap().clone();
        let last_video = got
            .iter()
            .rev()
            .find_map(|e| match e {
                RuntimeEvent::Video(v) => Some(v.clone()),
                _ => None,
            })
            .unwrap();
        // GPUのない環境では「使用できません」
        assert!(matches!(
            last_video,
            VideoStatus::Idle | VideoStatus::Unavailable
        ));
    }
}
