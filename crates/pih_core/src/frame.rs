//! 切り抜きと黒い画面・信号断の検出（`frame_processor`。取り込み方法によらず共通）
use std::time::Instant;

use crate::geometry::Rect;
use crate::video::Frame;

/// 1枚の映像の判定の結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub crop: Rect,
    /// スマホの画面が消えている（黒い画面）、または取り込み元が途絶えた
    pub screen_off: bool,
}

pub trait FrameProcessor: Send {
    /// 間引いた縮小画像で判定する
    fn analyze(&mut self, frame: &Frame, now: Instant) -> FrameInfo;
}
