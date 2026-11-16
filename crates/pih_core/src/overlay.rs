//! 表示（`overlay_presenter`）と、VR内のパネルとチップの寸法
use crate::Result;
use crate::geometry::{Pose, Quat, Rect, Vec3};
use crate::pose::PanelState;
use crate::video::Frame;

pub trait OverlayPresenter: Send {
    fn set_frame(&mut self, frame: &Frame, crop: Rect) -> Result<()>;
    fn set_state(&mut self, state: &PanelState) -> Result<()>;
    fn set_appearance(&mut self, size_inch: f32, brightness: f32) -> Result<()>;
    /// ホームの「VRに表示」のスイッチ
    fn set_enabled(&mut self, enabled: bool) -> Result<()>;
}

/// 追跡が切れたときのパネルの不透明度（S04cの「薄く表示」）
pub const DIMMED_ALPHA: f32 = 0.45;
/// 状態のチップの高さ（メートル）
pub const CHIP_HEIGHT_M: f32 = 0.014;
/// パネルの上端とチップの間（メートル）
pub const CHIP_GAP_M: f32 = 0.006;
/// チップをパネルより手前に出す距離（メートル。重なって見えなくならないように）
pub const CHIP_LIFT_M: f32 = 0.002;
/// 映像の大きさがまだ分からないときの、パネルの縦横比（幅÷高さ。モックのスマホの画面 9:19.5）
pub const DEFAULT_PANEL_ASPECT: f32 = 9.0 / 19.5;

const METERS_PER_INCH: f32 = 0.0254;

/// 切り抜いた映像の縦横比（幅÷高さ）
pub fn aspect(crop: Rect) -> f32 {
    if crop.width == 0 || crop.height == 0 {
        DEFAULT_PANEL_ASPECT
    } else {
        crop.width as f32 / crop.height as f32
    }
}

/// 対角の大きさ（インチ）と縦横比から、パネルの幅と高さ（メートル）
pub fn panel_size_m(size_inch: f32, aspect: f32) -> (f32, f32) {
    let diag = size_inch.max(0.0) * METERS_PER_INCH;
    let a = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        DEFAULT_PANEL_ASPECT
    };
    let height = diag / (1.0 + a * a).sqrt();
    (height * a, height)
}

/// チップの姿勢。パネルを出しているときは上端の上に、隠しているとき（画面が消えた）はパネルの中央に置く
pub fn chip_pose(panel: &Pose, panel_height_m: f32, panel_hidden: bool) -> Pose {
    let y = if panel_hidden {
        0.0
    } else {
        panel_height_m / 2.0 + CHIP_GAP_M + CHIP_HEIGHT_M / 2.0
    };
    panel.compose(&Pose::new(Vec3::new(0.0, y, CHIP_LIFT_M), Quat::IDENTITY))
}

/// チップの画像（幅×高さの画素）から、チップの幅（メートル）
pub fn chip_width_m(width_px: u32, height_px: u32) -> f32 {
    if height_px == 0 {
        return CHIP_HEIGHT_M;
    }
    CHIP_HEIGHT_M * width_px as f32 / height_px as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_size_keeps_diagonal_and_aspect() {
        let (w, h) = panel_size_m(6.7, 9.0 / 19.5);
        let diag = (w * w + h * h).sqrt();
        assert!((diag - 6.7 * 0.0254).abs() < 1e-5);
        assert!((w / h - 9.0 / 19.5).abs() < 1e-5);
        // 横長の映像でも対角は同じ
        let (w2, h2) = panel_size_m(6.7, 16.0 / 9.0);
        assert!(w2 > h2);
        assert!(((w2 * w2 + h2 * h2).sqrt() - diag).abs() < 1e-5);
    }

    #[test]
    fn aspect_falls_back_for_empty_crop() {
        assert_eq!(
            aspect(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0
            }),
            DEFAULT_PANEL_ASPECT
        );
        assert_eq!(
            aspect(Rect {
                x: 5,
                y: 5,
                width: 100,
                height: 50
            }),
            2.0
        );
    }

    #[test]
    fn chip_sits_above_the_panel_or_on_it_when_hidden() {
        let panel = Pose::new(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY);
        let above = chip_pose(&panel, 0.14, false);
        assert!((above.position.y - (1.0 + 0.07 + CHIP_GAP_M + CHIP_HEIGHT_M / 2.0)).abs() < 1e-6);
        let center = chip_pose(&panel, 0.14, true);
        assert!((center.position.y - 1.0).abs() < 1e-6);
        assert!(center.position.z > 0.0);
        assert!((chip_width_m(300, 50) - CHIP_HEIGHT_M * 6.0).abs() < 1e-6);
    }
}
