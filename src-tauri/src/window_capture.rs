//! `window_capture_source`（T3-2）：Windows.Graphics.Capture でウィンドウをGPUのテクスチャとして受け取る。
//!
//! - ウィンドウは設定の `video.window`（タイトルと実行ファイル名）で探す。タイトルが変わっても、
//!   同じ実行ファイルのウィンドウがあれば使う
//! - フレームはテクスチャのまま渡し、CPUを経由しない。縮小画像（判定とプレビュー用）だけを間引いて読み出す
//! - ウィンドウが閉じたら、状態を「取り込めない（source_lost）」にする
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use pih_core::geometry::Size;
use pih_core::video::{
    Frame, SharedTexture, VideoConfig, VideoError, VideoSource, VideoSourceKind, VideoStatus,
    WindowInfo,
};
use pih_core::{Error, Result};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GA_ROOTOWNER, GWL_EXSTYLE, GetAncestor, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, Interface};

use crate::gpu::{Gpu, ThumbnailReader};

/// 縮小画像を読み出す間隔（UIのプレビューは5fps程度。imple 2章）
pub const THUMBNAIL_INTERVAL: Duration = Duration::from_millis(200);
/// 縮小画像の幅の上限
pub const THUMBNAIL_MAX_WIDTH: u32 = 360;

// ---- ウィンドウの一覧 ----

/// 見つけたウィンドウ
#[derive(Debug, Clone)]
struct Found {
    hwnd: HWND,
    info: WindowInfo,
}

fn window_title(hwnd: HWND) -> Option<String> {
    // SAFETY: 有効なウィンドウのハンドルと、十分な大きさのバッファ
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize + 1];
    // SAFETY: 同上
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
    let title = title.trim().to_owned();
    (!title.is_empty()).then_some(title)
}

fn process_executable(pid: u32) -> Option<String> {
    // SAFETY: 情報の読み取りだけの権限で開き、必ず閉じる
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(h);
        ok.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
    }
}

/// 取り込みの対象にできるウィンドウか（見えている、ツールウィンドウでない、隠されていない、自分ではない）
fn capturable(hwnd: HWND, own_pid: u32) -> Option<Found> {
    // SAFETY: EnumWindows から受け取った有効なハンドル
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return None;
        }
        if GetAncestor(hwnd, GA_ROOTOWNER) != hwnd {
            return None;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 {
            return None;
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == own_pid {
            return None;
        }
        let title = window_title(hwnd)?;
        let executable = process_executable(pid).unwrap_or_default();
        Some(Found {
            hwnd,
            info: WindowInfo { title, executable },
        })
    }
}

fn enumerate() -> Vec<Found> {
    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam は enumerate の中の Vec を指す
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        out.push(hwnd);
        BOOL(1)
    }
    let mut hwnds: Vec<HWND> = Vec::new();
    // SAFETY: コールバックの間だけ有効なポインタを渡す
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut hwnds as *mut _ as isize));
    }
    // SAFETY: 引数なし
    let own = unsafe { GetCurrentProcessId() };
    hwnds
        .into_iter()
        .filter_map(|h| capturable(h, own))
        .collect()
}

/// 取り込めるウィンドウの一覧（`list_windows`）
pub fn list_windows() -> Result<Vec<WindowInfo>> {
    if !GraphicsCaptureSession::IsSupported().unwrap_or(false) {
        return Err(Error::Unavailable);
    }
    let mut list: Vec<WindowInfo> = enumerate().into_iter().map(|f| f.info).collect();
    list.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(list)
}

/// 設定のタイトルと実行ファイル名に最も合うウィンドウ
fn find(title: &str, executable: &str) -> Option<Found> {
    let all = enumerate();
    let exe_matches = |f: &&Found| !executable.is_empty() && f.info.executable == executable;
    all.iter()
        .find(|f| f.info.title == title && (executable.is_empty() || exe_matches(f)))
        .or_else(|| all.iter().find(exe_matches))
        .or_else(|| all.iter().find(|f| f.info.title == title))
        .cloned()
}

// ---- 取り込み ----

#[derive(Default)]
struct Signal {
    pending: bool,
    closed: bool,
}

type Shared = Arc<(Mutex<Signal>, Condvar)>;

struct Session {
    /// 閉じたときの通知を受けるため、取り込みの間は保持する
    _item: GraphicsCaptureItem,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    signal: Shared,
    pool_size: SizeInt32,
    title: String,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

/// 取り込んだ1枚。オーバーレイへ写し終えるまで、フレームをプールへ戻さない
struct CapturedFrame {
    frame: Direct3D11CaptureFrame,
    texture: ID3D11Texture2D,
}

// SAFETY: どちらもフリースレッドのオブジェクト（WGCのフレームとD3D11のテクスチャ）
unsafe impl Send for CapturedFrame {}
unsafe impl Sync for CapturedFrame {}

impl Drop for CapturedFrame {
    fn drop(&mut self) {
        let _ = self.frame.Close();
    }
}

impl CapturedFrame {
    fn texture_ptr(&self) -> usize {
        self.texture.as_raw() as usize
    }
}

pub struct WindowCaptureSource {
    gpu: Arc<Gpu>,
    session: Option<Session>,
    status: VideoStatus,
    reader: ThumbnailReader,
    last_thumbnail: Option<Instant>,
    fps_window_start: Instant,
    fps_count: u32,
    fps: f32,
}

impl WindowCaptureSource {
    pub fn new(gpu: Arc<Gpu>) -> Self {
        WindowCaptureSource {
            reader: ThumbnailReader::new(gpu.clone(), THUMBNAIL_MAX_WIDTH),
            gpu,
            session: None,
            status: VideoStatus::Idle,
            last_thumbnail: None,
            fps_window_start: Instant::now(),
            fps_count: 0,
            fps: 0.0,
        }
    }

    fn open(&self, found: &Found) -> windows::core::Result<Session> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        // SAFETY: 列挙で得た有効なウィンドウのハンドル
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(found.hwnd)? };
        let size = item.Size()?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &self.gpu.winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        // 古いWindowsにはないため、失敗しても続ける
        let _ = session.SetIsCursorCaptureEnabled(false);
        let _ = session.SetIsBorderRequired(false);

        let signal: Shared = Arc::default();
        let s = signal.clone();
        pool.FrameArrived(&TypedEventHandler::new(move |_, _| {
            let (lock, cv) = &*s;
            lock.lock().unwrap_or_else(|e| e.into_inner()).pending = true;
            cv.notify_all();
            Ok(())
        }))?;
        let s = signal.clone();
        item.Closed(&TypedEventHandler::new(move |_, _| {
            let (lock, cv) = &*s;
            lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
            cv.notify_all();
            Ok(())
        }))?;
        session.StartCapture()?;
        Ok(Session {
            _item: item,
            pool,
            session,
            signal,
            pool_size: size,
            title: found.info.title.clone(),
        })
    }

    fn count_frame(&mut self, now: Instant) {
        self.fps_count += 1;
        let elapsed = now.duration_since(self.fps_window_start);
        if elapsed >= Duration::from_secs(1) {
            self.fps = (self.fps_count as f32 / elapsed.as_secs_f32()).round();
            self.fps_count = 0;
            self.fps_window_start = now;
        }
    }

    /// 溜まっているフレームのうち、いちばん新しいもの
    fn latest_frame(session: &Session) -> Option<Direct3D11CaptureFrame> {
        let mut latest = None;
        while let Ok(f) = session.pool.TryGetNextFrame() {
            if let Some(old) = latest.replace(f) {
                let _ = old.Close();
            }
        }
        latest
    }
}

impl VideoSource for WindowCaptureSource {
    fn kind(&self) -> VideoSourceKind {
        VideoSourceKind::Window
    }

    fn start(&mut self, config: &VideoConfig) -> Result<()> {
        self.stop();
        let VideoConfig::Window { title, executable } = config else {
            return Err(Error::Invalid("video config".into()));
        };
        let Some(found) = find(title, executable) else {
            self.status = VideoStatus::Error {
                error: VideoError::OpenFailed,
            };
            return Err(Error::NotFound(title.clone()));
        };
        match self.open(&found) {
            Ok(session) => {
                self.session = Some(session);
                self.status = VideoStatus::NoSignal;
                self.fps_window_start = Instant::now();
                self.fps_count = 0;
                self.fps = 0.0;
                Ok(())
            }
            Err(e) => {
                log::warn!("ウィンドウを取り込めません: {e}");
                self.status = VideoStatus::Error {
                    error: VideoError::OpenFailed,
                };
                Err(Error::Unavailable)
            }
        }
    }

    fn next_frame(&mut self, timeout: Duration) -> Result<Option<Frame>> {
        let Some(session) = self.session.as_ref() else {
            return Err(Error::Unavailable);
        };
        let closed = {
            let (lock, cv) = &*session.signal;
            let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            let (mut guard, _) = cv
                .wait_timeout_while(guard, timeout, |s| !s.pending && !s.closed)
                .unwrap_or_else(|e| e.into_inner());
            guard.pending = false;
            guard.closed
        };
        if closed {
            self.session = None;
            self.status = VideoStatus::Error {
                error: VideoError::SourceLost,
            };
            return Ok(None);
        }
        let Some(frame) = Self::latest_frame(session) else {
            return Ok(None);
        };

        let content = frame.ContentSize().map_err(|_| Error::Unavailable)?;
        // ウィンドウの大きさが変わったら、プールを作り直す（このフレームはそのまま使う）
        if content.Width != session.pool_size.Width || content.Height != session.pool_size.Height {
            if content.Width > 0 && content.Height > 0 {
                let _ = session.pool.Recreate(
                    &self.gpu.winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    content,
                );
            }
            if let Some(s) = self.session.as_mut() {
                s.pool_size = content;
            }
        }
        let title = self
            .session
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();

        let texture: ID3D11Texture2D = frame
            .Surface()
            .and_then(|s| s.cast::<IDirect3DDxgiInterfaceAccess>())
            // SAFETY: WGCのサーフェスはD3D11のテクスチャを持つ
            .and_then(|a| unsafe { a.GetInterface::<ID3D11Texture2D>() })
            .map_err(|e| {
                log::warn!("フレームのテクスチャを得られません: {e}");
                Error::Unavailable
            })?;
        let tex_size = crate::gpu::texture_size(&texture);
        let size = Size {
            width: (content.Width.max(0) as u32).min(tex_size.width),
            height: (content.Height.max(0) as u32).min(tex_size.height),
        };
        if size.width == 0 || size.height == 0 {
            return Ok(None);
        }

        let now = Instant::now();
        self.count_frame(now);
        let thumbnail = if self
            .last_thumbnail
            .is_none_or(|t| now.duration_since(t) >= THUMBNAIL_INTERVAL)
        {
            self.last_thumbnail = Some(now);
            match self.reader.read(&texture, size) {
                Ok(t) => Some(Arc::new(t)),
                Err(e) => {
                    log::debug!("縮小画像を読み出せません: {e}");
                    None
                }
            }
        } else {
            None
        };

        self.status = VideoStatus::Running {
            device: title,
            size,
            fps: self.fps,
        };
        let captured = Arc::new(CapturedFrame { frame, texture });
        Ok(Some(Frame::Gpu {
            texture: SharedTexture {
                ptr: captured.texture_ptr(),
                owner: captured,
            },
            size,
            thumbnail,
        }))
    }

    fn stop(&mut self) {
        self.session = None;
        self.status = VideoStatus::Idle;
    }

    fn status(&self) -> VideoStatus {
        self.status.clone()
    }
}

impl Drop for WindowCaptureSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_excludes_own_windows_and_untitled() {
        // デスクトップのセッションがない環境（CI）では一覧が空でもよい。失敗しないこと
        if let Ok(list) = list_windows() {
            assert!(list.iter().all(|w| !w.title.is_empty()));
        }
    }

    /// デスクトップの実際のウィンドウを取り込む（手元での確認用。`cargo test -- --ignored`）
    #[test]
    #[ignore]
    fn captures_a_real_window() {
        let gpu = Gpu::new().unwrap();
        let list = list_windows().unwrap();
        let mut src = WindowCaptureSource::new(gpu);
        let w = list.first().expect("取り込めるウィンドウがない");
        src.start(&VideoConfig::Window {
            title: w.title.clone(),
            executable: w.executable.clone(),
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let frame = loop {
            if let Some(f) = src.next_frame(Duration::from_millis(200)).unwrap() {
                break f;
            }
            assert!(Instant::now() < deadline, "フレームが届かない");
        };
        let Frame::Gpu {
            size, thumbnail, ..
        } = &frame
        else {
            panic!("GPUのフレームではない");
        };
        let t = thumbnail
            .as_ref()
            .expect("最初のフレームには縮小画像が付く");
        assert!(t.size.width <= THUMBNAIL_MAX_WIDTH && t.size.width > 0);
        assert_eq!(t.bgra.len(), (t.size.width * t.size.height * 4) as usize);
        let Frame::Gpu { texture, .. } = &frame else {
            unreachable!()
        };
        assert!(texture.owner.downcast_ref::<CapturedFrame>().is_some());
        assert!(matches!(src.status(), VideoStatus::Running { .. }));
        eprintln!(
            "{} / {} -> {size:?}, 縮小 {:?}",
            w.title, w.executable, t.size
        );
    }

    #[test]
    fn start_with_unknown_window_reports_open_failed() {
        let Ok(gpu) = Gpu::new() else {
            return; // GPUのない環境
        };
        let mut src = WindowCaptureSource::new(gpu);
        let r = src.start(&VideoConfig::Window {
            title: "phone_in_hand のテスト用の存在しないウィンドウ".into(),
            executable: "no_such_program.exe".into(),
        });
        assert!(matches!(r, Err(Error::NotFound(_))));
        assert_eq!(
            src.status(),
            VideoStatus::Error {
                error: VideoError::OpenFailed
            }
        );
        assert!(matches!(
            src.next_frame(Duration::ZERO),
            Err(Error::Unavailable)
        ));
    }
}
