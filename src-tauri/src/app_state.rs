//! `app_state`：各モジュールの状態をまとめて `AppSnapshot` を持ち、変化のたびに通知する。
//! UIからの操作は、Tauriのコマンド（`commands.rs`）がここへ渡す。
//!
//! - 通知の先は関数で受け取り、Tauriに依存せずにテストできるようにする
//! - 映像と姿勢のスレッド（`runtime.rs`）へは [`RuntimeLink`] で伝え、スレッドからは
//!   [`AppState::on_runtime_event`] で知らせを受ける。`RuntimeLink` は自分のロックの外で呼ぶ
use std::sync::{Mutex, OnceLock};

use pih_core::adb::{AdbController, AdbTarget, DeviceInfo};
use pih_core::calibration::CalibrationProgress;
use pih_core::fake::{FakeAdbController, FakeVrchatBridge};
use pih_core::pose::effective_offset;
use pih_core::settings::{Calibration, PoseSettings, Settings, Stored, VideoSettings};
use pih_core::settings_store::SettingsStore;
use pih_core::snapshot::{AppSnapshot, TrackingStatus};
use pih_core::video::{VideoStatus, WindowInfo};
use pih_core::vrchat::VrchatBridge;
use pih_core::{Error, Result};

use crate::overlay::OverlayImages;
use crate::runtime::{RuntimeEvent, RuntimeLink};

type Notify = Box<dyn Fn(&AppSnapshot) + Send + Sync>;
type NotifyProgress = Box<dyn Fn(CalibrationProgress) + Send + Sync>;

/// 各モジュールの実装。未実装のものは偽の実装（「使用できません」を返す）
pub struct Modules {
    pub adb: Box<dyn AdbController>,
    pub vrchat: Box<dyn VrchatBridge>,
}

impl Default for Modules {
    fn default() -> Self {
        Modules {
            adb: Box::new(FakeAdbController),
            vrchat: Box::new(FakeVrchatBridge::default()),
        }
    }
}

struct Inner {
    settings: Settings,
    calibration: Option<Calibration>,
    snapshot: AppSnapshot,
    store: Box<dyn SettingsStore>,
    modules: Modules,
    /// 映像のスレッドが知らせた状態（スレッドがなければ「使用できません」）
    video: VideoStatus,
    /// 姿勢のスレッドが知らせた状態
    tracking: TrackingStatus,
}

pub struct AppState {
    inner: Mutex<Inner>,
    runtime: OnceLock<Box<dyn RuntimeLink>>,
    notify: Notify,
    /// 位置合わせの進み具合（`calibration_progress`）
    progress: NotifyProgress,
}

impl AppState {
    /// 保存したものを読み込んで始める。読めないファイルは退避済みで、既定値で続ける
    pub fn new(
        store: Box<dyn SettingsStore>,
        modules: Modules,
        notify: Notify,
        progress: NotifyProgress,
    ) -> Result<Self> {
        let Stored {
            settings,
            calibration,
            recovered,
            ..
        } = store.load()?;
        for name in recovered {
            log::warn!("{name} を読めなかったため、{name}.broken に退避して既定値で起動します");
        }
        let snapshot = AppSnapshot::initial(&settings, calibration.as_ref());
        let state = AppState {
            inner: Mutex::new(Inner {
                settings,
                calibration,
                snapshot,
                store,
                modules,
                video: VideoStatus::Unavailable,
                tracking: TrackingStatus::Unavailable,
            }),
            runtime: OnceLock::new(),
            notify,
            progress,
        };
        state.with(|inner| inner.recompute());
        Ok(state)
    }

    /// 映像と姿勢のスレッドをつなぐ（起動時に1回）。今の設定を伝える
    pub fn attach_runtime(&self, runtime: Box<dyn RuntimeLink>) {
        if self.runtime.set(runtime).is_err() {
            log::warn!("スレッドはすでにつながっています");
        }
        self.push_config();
    }

    fn runtime(&self) -> Option<&dyn RuntimeLink> {
        self.runtime.get().map(|r| r.as_ref())
    }

    /// 今の設定と位置合わせの結果をスレッドへ伝える（自分のロックの外で呼ぶ）
    fn push_config(&self) {
        let (settings, calibration) = self.with(|i| (i.settings.clone(), i.calibration));
        if let Some(rt) = self.runtime() {
            rt.configure(&settings, calibration.as_ref());
        }
    }

    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = inner.snapshot.clone();
        let out = f(&mut inner);
        inner.recompute();
        if inner.snapshot != before {
            (self.notify)(&inner.snapshot);
        }
        out
    }

    pub fn snapshot(&self) -> AppSnapshot {
        self.with(|i| i.snapshot.clone())
    }

    pub fn settings(&self) -> Settings {
        self.with(|i| i.settings.clone())
    }

    /// 各スレッドからの知らせ
    pub fn on_runtime_event(&self, ev: RuntimeEvent) {
        match ev {
            RuntimeEvent::Video(v) => self.with(|i| i.video = v),
            RuntimeEvent::Tracking(t) => self.with(|i| i.tracking = t),
            RuntimeEvent::SteamVr(connected) => {
                // つながったら、SteamVRと一緒に起動する設定を登録に合わせる
                if connected {
                    let want = self.with(|i| i.settings.general.start_with_steamvr);
                    if let Some(rt) = self.runtime() {
                        rt.sync_startup(want);
                    }
                }
            }
            RuntimeEvent::CalibrationFinished(c) => {
                let outcome = self.save_calibration(c);
                (self.progress)(match outcome {
                    Ok(()) => CalibrationProgress::Finished,
                    Err(e) => {
                        log::warn!("位置合わせの結果を保存できません: {e}");
                        CalibrationProgress::Failed
                    }
                });
            }
            RuntimeEvent::CalibrationFailed => (self.progress)(CalibrationProgress::Failed),
        }
    }

    /// 位置合わせを始める。基準と側は、位置合わせのページで選んだもの（終えたときに保存する）
    pub fn start_calibration(&self, pose: PoseSettings) -> Result<()> {
        let start = self.with(|i| effective_offset(i.calibration.as_ref(), &pose));
        let rt = self.runtime().ok_or(Error::Unavailable)?;
        rt.start_calibration(pose, start)?;
        (self.progress)(CalibrationProgress::Running);
        Ok(())
    }

    /// 位置合わせをやめる。前の結果はそのまま残す
    pub fn cancel_calibration(&self) {
        if let Some(rt) = self.runtime() {
            rt.cancel_calibration();
        }
        (self.progress)(CalibrationProgress::Cancelled);
    }

    /// 位置合わせを、今の見本の位置で決定する
    pub fn finish_calibration(&self) -> Result<()> {
        let rt = self.runtime().ok_or(Error::Unavailable)?;
        let c = rt.finish_calibration()?;
        self.save_calibration(c)?;
        (self.progress)(CalibrationProgress::Finished);
        Ok(())
    }

    /// 位置合わせの結果を `calibration.json` へ上書きし、基準と側を設定に保存する
    fn save_calibration(&self, c: Calibration) -> Result<()> {
        self.with(|i| {
            let mut next = i.settings.clone();
            next.pose.source = c.source;
            next.pose.side = c.side;
            i.store.save_calibration(&c)?;
            i.calibration = Some(c);
            if next != i.settings {
                i.store.save_settings(&next)?;
                i.settings = next;
            }
            Ok::<_, Error>(())
        })?;
        self.push_config();
        Ok(())
    }

    /// 部分的な変更を当てて保存する（設定の「保存」）
    pub fn update_settings(&self, patch: &serde_json::Value) -> Result<Settings> {
        let (saved, startup_changed) = self.with(|i| {
            let next = i.settings.patched(patch)?;
            i.store.save_settings(&next)?;
            let changed = next.general.start_with_steamvr != i.settings.general.start_with_steamvr;
            i.settings = next;
            Ok::<_, Error>((i.settings.clone(), changed))
        })?;
        self.push_config();
        if startup_changed && let Some(rt) = self.runtime() {
            rt.sync_startup(saved.general.start_with_steamvr);
        }
        Ok(saved)
    }

    /// ホームの「VRに表示」。保存し、オーバーレイとVRChat（表示中のパラメータ）へ伝える
    pub fn set_display_enabled(&self, enabled: bool) -> Result<AppSnapshot> {
        self.with(|i| {
            let mut next = i.settings.clone();
            next.display.enabled = enabled;
            i.store.save_settings(&next)?;
            i.settings = next;
            if let Err(e) = i.modules.vrchat.send_visible(enabled) {
                log::warn!("VRChatへ表示中を送れません: {e}");
            }
            Ok::<_, Error>(())
        })?;
        self.push_config();
        Ok(self.snapshot())
    }

    /// セットアップの終わり：ここまでの設定と `setup_completed=true` をまとめて保存する
    pub fn complete_setup(&self, patch: &serde_json::Value) -> Result<Settings> {
        let saved = self.with(|i| {
            let mut next = i.settings.patched(patch)?;
            next.setup_completed = true;
            i.store.save_settings(&next)?;
            i.settings = next;
            Ok::<_, Error>(i.settings.clone())
        })?;
        self.push_config();
        Ok(saved)
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        self.runtime()
            .ok_or(Error::Unavailable)
            .and_then(|r| r.list_windows())
    }

    /// 保存していない映像の設定で取り込む（映像のページのプレビュー）。
    /// 保存せずにページを離れるときは、保存済みの設定でもう一度呼ぶ
    pub fn start_video(&self, video: &VideoSettings) -> Result<()> {
        let rt = self.runtime().ok_or(Error::Unavailable)?;
        rt.preview_video(video);
        Ok(())
    }

    pub fn set_overlay_images(&self, images: OverlayImages) -> Result<()> {
        let rt = self.runtime().ok_or(Error::Unavailable)?;
        rt.set_overlay_images(images);
        Ok(())
    }

    pub fn adb_pair(&self, host: &str, port: u16, code: &str) -> Result<()> {
        self.with(|i| i.modules.adb.pair(host, port, code))
    }

    pub fn adb_connect(&self, target: &AdbTarget) -> Result<DeviceInfo> {
        self.with(|i| i.modules.adb.connect(target))
    }
}

impl Inner {
    /// 設定と各モジュールの状態から `AppSnapshot` を作り直す
    fn recompute(&mut self) {
        let mut snap = AppSnapshot::initial(&self.settings, self.calibration.as_ref());
        snap.video = self.video.clone();
        snap.tracking = self.tracking.clone();
        self.snapshot = snap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::NullRuntime;
    use pih_core::fake::MemorySettingsStore;
    use pih_core::geometry::Side;
    use pih_core::pose::PanelMode;
    use pih_core::snapshot::{AdbStatus, CalibrationStatus};
    use pih_core::video::VideoSourceKind;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type Progress = Arc<std::sync::Mutex<Vec<CalibrationProgress>>>;

    fn state_with_progress() -> (AppState, Arc<AtomicUsize>, Arc<NullRuntime>, Progress) {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let notify: Notify = Box::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let progress: Progress = Arc::default();
        let p = progress.clone();
        let s = AppState::new(
            Box::new(MemorySettingsStore::default()),
            Modules::default(),
            notify,
            Box::new(move |e| p.lock().unwrap().push(e)),
        )
        .unwrap();
        let rt = Arc::new(NullRuntime::default());
        s.attach_runtime(Box::new(rt.clone()));
        (s, count, rt, progress)
    }

    fn state() -> (AppState, Arc<AtomicUsize>, Arc<NullRuntime>) {
        let (s, c, rt, _) = state_with_progress();
        (s, c, rt)
    }

    #[test]
    fn calibration_finish_saves_result_and_pose() {
        let (s, _, rt, progress) = state_with_progress();
        let pose = PoseSettings {
            source: pih_core::settings::PoseSourceKind::Controller,
            side: Side::Left,
        };
        s.start_calibration(pose).unwrap();
        // 前の結果がなければ、既定のずれから始める
        assert_eq!(
            rt.calibrating.lock().unwrap().unwrap().1,
            pih_core::pose::default_offset()
        );
        s.finish_calibration().unwrap();
        assert_eq!(s.snapshot().calibration, CalibrationStatus::Valid);
        assert_eq!(s.settings().pose, pose);
        assert_eq!(
            *progress.lock().unwrap(),
            vec![CalibrationProgress::Running, CalibrationProgress::Finished]
        );
        // スレッドへ新しいずれを伝えた
        assert_eq!(rt.configured.lock().unwrap().last().unwrap().pose, pose);
    }

    #[test]
    fn calibration_cancel_keeps_previous_result() {
        let (s, _, _, progress) = state_with_progress();
        let pose = s.settings().pose;
        s.start_calibration(pose).unwrap();
        s.finish_calibration().unwrap();
        s.start_calibration(pose).unwrap();
        s.cancel_calibration();
        assert_eq!(s.snapshot().calibration, CalibrationStatus::Valid);
        assert!(matches!(s.finish_calibration(), Err(Error::Unavailable)));
        assert_eq!(
            progress.lock().unwrap().last(),
            Some(&CalibrationProgress::Cancelled)
        );
    }

    #[test]
    fn start_with_steamvr_is_synced_on_change_and_on_connect() {
        let (s, _, rt) = state();
        // ほかの設定の変更では合わせない
        s.update_settings(&json!({ "display": { "brightness": 0.5 } }))
            .unwrap();
        assert!(rt.startup.lock().unwrap().is_empty());
        s.update_settings(&json!({ "general": { "start_with_steamvr": true } }))
            .unwrap();
        s.on_runtime_event(RuntimeEvent::SteamVr(true));
        s.on_runtime_event(RuntimeEvent::SteamVr(false));
        s.update_settings(&json!({ "general": { "start_with_steamvr": false } }))
            .unwrap();
        assert_eq!(*rt.startup.lock().unwrap(), vec![true, true, false]);
    }

    #[test]
    fn calibration_finished_in_vr_is_saved() {
        let (s, _, _, progress) = state_with_progress();
        let c = Calibration {
            source: pih_core::settings::PoseSourceKind::Controller,
            side: Side::Right,
            offset: pih_core::geometry::Pose::default(),
            calibrated_at_ms: 1,
        };
        s.on_runtime_event(RuntimeEvent::CalibrationFinished(c));
        assert_eq!(s.snapshot().calibration, CalibrationStatus::Valid);
        s.on_runtime_event(RuntimeEvent::CalibrationFailed);
        assert_eq!(
            *progress.lock().unwrap(),
            vec![CalibrationProgress::Finished, CalibrationProgress::Failed]
        );
        // 側を変えると、前の結果は使えない（合わせ直しを促す）
        s.update_settings(&json!({ "pose": { "side": "left" } }))
            .unwrap();
        assert_eq!(s.snapshot().calibration, CalibrationStatus::Stale);
    }

    #[test]
    fn starts_with_unavailable_devices() {
        let (s, _, rt) = state();
        let snap = s.snapshot();
        assert_eq!(snap.video, VideoStatus::Unavailable);
        assert_eq!(snap.adb, AdbStatus::Unavailable);
        assert_eq!(snap.calibration, CalibrationStatus::Missing);
        assert!(!s.settings().setup_completed);
        // つないだ時点の設定を伝える
        assert_eq!(rt.configured.lock().unwrap().len(), 1);
    }

    #[test]
    fn display_toggle_saves_notifies_and_configures() {
        let (s, count, rt) = state();
        let snap = s.set_display_enabled(true).unwrap();
        assert!(snap.display_enabled);
        assert!(s.settings().display.enabled);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert!(
            rt.configured
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .display
                .enabled
        );
        // 同じ値をもう一度：状態は変わらず、通知もしない
        s.set_display_enabled(true).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn settings_change_updates_derived_state() {
        let (s, count, rt) = state();
        s.update_settings(&json!({ "phone_kind": "iphone" }))
            .unwrap();
        assert_eq!(s.snapshot().adb, AdbStatus::NotUsed);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(rt.configured.lock().unwrap().len(), 2);
    }

    #[test]
    fn invalid_patch_changes_nothing() {
        let (s, count, rt) = state();
        let before = s.settings();
        assert!(
            s.update_settings(&json!({ "display": { "brightness": -1 } }))
                .is_err()
        );
        assert_eq!(s.settings(), before);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert_eq!(rt.configured.lock().unwrap().len(), 1);
    }

    #[test]
    fn complete_setup_saves_everything_at_once() {
        let (s, _, _) = state();
        let saved = s
            .complete_setup(&json!({ "video": { "source": "window" } }))
            .unwrap();
        assert!(saved.setup_completed);
        assert_eq!(saved.video.source, VideoSourceKind::Window);
    }

    #[test]
    fn runtime_events_update_snapshot() {
        let (s, count, _) = state();
        s.on_runtime_event(RuntimeEvent::Video(VideoStatus::Idle));
        s.on_runtime_event(RuntimeEvent::Tracking(TrackingStatus::Tracking {
            mode: PanelMode::Controller,
            side: Side::Right,
        }));
        let snap = s.snapshot();
        assert_eq!(snap.video, VideoStatus::Idle);
        assert!(matches!(snap.tracking, TrackingStatus::Tracking { .. }));
        assert_eq!(count.load(Ordering::SeqCst), 2);
        // 同じ知らせでは通知しない
        s.on_runtime_event(RuntimeEvent::Video(VideoStatus::Idle));
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn preview_video_does_not_save() {
        let (s, _, rt) = state();
        let mut v = s.settings().video;
        v.source = VideoSourceKind::Window;
        v.window.title = Some("AirPlay".into());
        s.start_video(&v).unwrap();
        assert_eq!(rt.previews.lock().unwrap().len(), 1);
        assert_eq!(s.settings().video.source, VideoSourceKind::Hdmi);
    }

    #[test]
    fn unimplemented_commands_report_unavailable() {
        let (s, _, _) = state();
        assert!(matches!(s.list_windows(), Err(Error::Unavailable)));
        assert!(matches!(
            s.adb_pair("192.168.0.2", 37000, "123456"),
            Err(Error::Unavailable)
        ));
    }
}
