//! 表示（`overlay_presenter`）
use crate::Result;
use crate::geometry::Rect;
use crate::pose::PanelState;
use crate::video::Frame;

pub trait OverlayPresenter: Send {
    fn set_frame(&mut self, frame: &Frame, crop: Rect) -> Result<()>;
    fn set_state(&mut self, state: &PanelState) -> Result<()>;
    fn set_appearance(&mut self, size_inch: f32, brightness: f32) -> Result<()>;
    /// ホームの「VRに表示」のスイッチ
    fn set_enabled(&mut self, enabled: bool) -> Result<()>;
}
