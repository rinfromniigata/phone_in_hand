//! 映像の取り込み（`video_source`）
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::geometry::Size;

/// D3D11のテクスチャ。`src-tauri` の実装が作る、同じプロセスの中で有効な `ID3D11Texture2D` のポインタ。
/// `owner`（取り込みのフレームなど）が生きている間だけ有効で、受け取った側は保持し続けない
#[derive(Clone)]
pub struct SharedTexture {
    pub ptr: usize,
    pub owner: Arc<dyn Any + Send + Sync>,
}

impl std::fmt::Debug for SharedTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedTexture")
            .field("ptr", &format_args!("{:#x}", self.ptr))
            .finish()
    }
}

/// 間引いて縮小した画像（BGRA、1画素4バイト、行の詰めなし）。
/// 黒い画面と余白の判定（`frame_processor`）と、UIのプレビューに使う
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub size: Size,
    pub bgra: Vec<u8>,
}

impl Thumbnail {
    /// 画素の明るさ（0〜255。BT.601の近似）
    pub fn luma(&self, x: u32, y: u32) -> u8 {
        let i = ((y * self.size.width + x) * 4) as usize;
        let (b, g, r) = (
            self.bgra[i] as u32,
            self.bgra[i + 1] as u32,
            self.bgra[i + 2] as u32,
        );
        ((r * 77 + g * 150 + b * 29) >> 8) as u8
    }

    /// CPUのフレームから、幅が `max_width` 以下になるよう間引いて作る。MJPEGは扱わない（`None`）
    pub fn from_cpu(format: PixelFormat, size: Size, data: &[u8], max_width: u32) -> Option<Self> {
        if size.width == 0 || size.height == 0 || max_width == 0 {
            return None;
        }
        let step = size.width.div_ceil(max_width).max(1);
        let tw = size.width / step;
        let th = size.height / step;
        if tw == 0 || th == 0 {
            return None;
        }
        let (w, h) = (size.width as usize, size.height as usize);
        let needed = match format {
            PixelFormat::Bgra8 => w * h * 4,
            PixelFormat::Yuy2 => w * h * 2,
            PixelFormat::Nv12 => w * h * 3 / 2,
            PixelFormat::Mjpeg => return None,
        };
        if data.len() < needed {
            return None;
        }
        let mut bgra = Vec::with_capacity((tw * th * 4) as usize);
        for ty in 0..th {
            for tx in 0..tw {
                let (x, y) = ((tx * step) as usize, (ty * step) as usize);
                let px = match format {
                    PixelFormat::Bgra8 => {
                        let i = (y * w + x) * 4;
                        [data[i], data[i + 1], data[i + 2], 255]
                    }
                    // 判定には明るさだけを使うため、灰色にする
                    PixelFormat::Yuy2 => {
                        let l = data[(y * w + x) * 2];
                        [l, l, l, 255]
                    }
                    PixelFormat::Nv12 => {
                        let l = data[y * w + x];
                        [l, l, l, 255]
                    }
                    PixelFormat::Mjpeg => unreachable!(),
                };
                bgra.extend_from_slice(&px);
            }
        }
        Some(Thumbnail {
            size: Size {
                width: tw,
                height: th,
            },
            bgra,
        })
    }
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
    /// D3D11のテクスチャ（WGC、MFデコーダー）。縮小画像は間引いたフレームにだけ付く
    Gpu {
        texture: SharedTexture,
        size: Size,
        thumbnail: Option<Arc<Thumbnail>>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_from_bgra_is_subsampled() {
        let size = Size {
            width: 8,
            height: 4,
        };
        // 左半分が白、右半分が黒
        let mut data = vec![0u8; 8 * 4 * 4];
        for y in 0..4 {
            for x in 0..4 {
                let i = (y * 8 + x) * 4;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let t = Thumbnail::from_cpu(PixelFormat::Bgra8, size, &data, 4).unwrap();
        assert_eq!(
            t.size,
            Size {
                width: 4,
                height: 2
            }
        );
        assert_eq!(t.luma(0, 0), 255);
        assert_eq!(t.luma(3, 1), 0);
    }

    #[test]
    fn thumbnail_rejects_short_data_and_mjpeg() {
        let size = Size {
            width: 4,
            height: 4,
        };
        assert!(Thumbnail::from_cpu(PixelFormat::Nv12, size, &[0; 10], 4).is_none());
        assert!(Thumbnail::from_cpu(PixelFormat::Mjpeg, size, &[0; 100], 4).is_none());
        let t = Thumbnail::from_cpu(PixelFormat::Nv12, size, &[200; 24], 4).unwrap();
        assert_eq!(t.luma(1, 1), 200);
    }
}
