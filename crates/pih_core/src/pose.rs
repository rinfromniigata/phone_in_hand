//! 姿勢（`pose_source`、`phone_orientation_source`、`pose_fusion`）
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::geometry::{Pose, Quat, Side};
use crate::settings::Calibration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingState {
    Tracking,
    Lost,
}

/// 基準（コントローラーか手）の姿勢と追跡状態
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseSample {
    pub pose: Option<Pose>,
    pub state: TrackingState,
}

pub trait PoseSource: Send {
    fn sample(&mut self, side: Side) -> PoseSample;
}

/// スマホの回転センサーの値（ハンドトラッキングのときだけ）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientationSample {
    pub rotation: Quat,
    pub received_at: Instant,
}

pub trait PhoneOrientationSource: Send {
    fn latest(&self) -> Option<OrientationSample>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelVisibility {
    Shown,
    Dimmed,
    Hidden,
}

/// パネルの姿勢の求め方
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelMode {
    /// コントローラーに固定
    Controller,
    /// 手の位置＋スマホの回転
    Hand,
    /// センサーが途絶え、手の向きで代用している
    HandFallback,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelState {
    pub pose: Pose,
    pub visibility: PanelVisibility,
    pub mode: PanelMode,
}

/// パネルの姿勢を求める。ヨーの補正、センサー途絶時の代用、追跡が切れたときの固定も受け持つ
///
/// 振る舞いはT3-4（コントローラー固定）とT9-3（ハンドトラッキング）で実装し、単体テストで固定する。
#[derive(Debug, Default)]
pub struct PoseFusion {
    last_pose: Option<Pose>,
}

impl PoseFusion {
    pub fn new() -> Self {
        Self::default()
    }

    /// T3-4で実装するまでは、常に非表示を返す
    pub fn update(
        &mut self,
        _base: &PoseSample,
        _orientation: Option<&OrientationSample>,
        _calibration: &Calibration,
        _screen_off: bool,
        _now: Instant,
    ) -> PanelState {
        PanelState {
            pose: self.last_pose.unwrap_or_default(),
            visibility: PanelVisibility::Hidden,
            mode: PanelMode::Controller,
        }
    }
}
