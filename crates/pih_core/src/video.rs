//! 映像の取り込み（`video_source`）
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::geometry::Size;

/// D3D11の共有テクスチャ。`src-tauri` の実装が作る、プロセス内で有効なハンドル（DXGIの共有ハンドルの値）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedTexture {
    pub handle: u64,
}

/// CPUのフレームの画素の形式（UVC）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    Mjpeg,
    Nv12,
    Yuy2,
    Bgra8,
}

/// 1枚の映像。GPUのフレームはテクスチャのまま渡し、CPUを経由しない
#[derive(Debug, Clone)]
pub enum Frame {
    /// D3D11の共有テクスチャ（WGC、MFデコーダー）
    Gpu { texture: SharedTexture, size: Size },
    /// UVC（MJPEG・NV12・YUY2など）。GPUへのアップロードは1回だけにする
    Cpu {
        format: PixelFormat,
        size: Size,
        data: Arc<[u8]>,
    },
}

impl Frame {
    pub fn size(&self) -> Size {
        match self {
            Frame::Gpu { size, .. } | Frame::Cpu { size, .. } => *size,
        }
    }
}

/// 取り込み方法（設定の `video.source`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoSourceKind {
    #[default]
    Hdmi,
    Mirror,
    Window,
}

/// 映像の取り込みの失敗の種類
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoError {
    /// 取り込み元が途絶えた
    SourceLost,
    /// 機器やウィンドウを開けない
    OpenFailed,
}

/// 映像の取り込みの状態（UIの状態の行に対応させる）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VideoStatus {
    /// この環境では使えない、またはまだ実装していない
    Unavailable,
    Idle,
    NoDevice,
    NoSignal,
    Running {
        device: String,
        size: Size,
        fps: f32,
    },
    Error {
        error: VideoError,
    },
}

/// 取り込みの開始に使う設定（各取り込み方法の設定から作る）
#[derive(Debug, Clone, PartialEq)]
pub enum VideoConfig {
    Hdmi {
        device_id: String,
        size: Size,
        fps: u32,
    },
    Mirror {
        serial: Option<String>,
    },
    Window {
        title: String,
        executable: String,
    },
}

pub trait VideoSource: Send {
    fn kind(&self) -> VideoSourceKind;
    fn start(&mut self, config: &VideoConfig) -> Result<()>;
    /// `None` はタイムアウト
    fn next_frame(&mut self, timeout: Duration) -> Result<Option<Frame>>;
    fn stop(&mut self);
    fn status(&self) -> VideoStatus;
}

/// 取り込み機器の一覧の1件（`list_capture_devices`）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptureDevice {
    pub id: String,
    pub name: String,
    pub modes: Vec<CaptureMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CaptureMode {
    pub size: Size,
    pub fps: u32,
}

/// 取り込めるウィンドウの1件（`list_windows`）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowInfo {
    pub title: String,
    pub executable: String,
}
