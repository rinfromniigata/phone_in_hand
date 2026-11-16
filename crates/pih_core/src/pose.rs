//! 姿勢（`pose_source`、`phone_orientation_source`、`pose_fusion`）
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::geometry::{Pose, Quat, Side, Vec3};
use crate::settings::{Calibration, PoseSettings};

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

impl PoseSample {
    /// 追跡中で姿勢があれば、その姿勢
    pub fn tracked(&self) -> Option<Pose> {
        match self.state {
            TrackingState::Tracking => self.pose,
            TrackingState::Lost => None,
        }
    }
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
    /// スマホの画面が消えている（隠している理由がそれか）
    pub screen_off: bool,
    /// 一度でも姿勢が得られたか（`pose` に意味があるか）
    pub has_pose: bool,
}

/// VR内の状態のチップ（S04）。名前は辞書のキー `vr.chip.*` に対応する
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chip {
    Tracking,
    SensorLost,
    HandLost,
    ScreenOff,
}

impl Chip {
    pub const ALL: [Chip; 4] = [
        Chip::Tracking,
        Chip::SensorLost,
        Chip::HandLost,
        Chip::ScreenOff,
    ];
}

impl PanelState {
    /// 出すチップ。まだ一度も姿勢が得られていない（出す場所がない）ときは出さない
    pub fn chip(&self) -> Option<Chip> {
        if !self.has_pose {
            return None;
        }
        if self.screen_off {
            return Some(Chip::ScreenOff);
        }
        match self.visibility {
            PanelVisibility::Hidden => None,
            PanelVisibility::Dimmed => Some(Chip::HandLost),
            PanelVisibility::Shown => Some(match self.mode {
                PanelMode::Controller | PanelMode::Hand => Chip::Tracking,
                PanelMode::HandFallback => Chip::SensorLost,
            }),
        }
    }
}

/// 位置合わせの結果がないとき（または基準や側を変えた後）の、コントローラーに対するパネルのずれ。
/// コントローラーの少し上に、上端を向こう側へ傾けて置く（OpenVRのコントローラーの座標：+Yが上、-Zが前）
pub fn default_offset() -> Pose {
    Pose::new(
        Vec3::new(0.0, 0.07, 0.0),
        Quat::from_rotation_x(-20f32.to_radians()),
    )
}

/// 使う位置合わせのずれ。基準と側が今の設定と合う結果がなければ、既定のずれ
pub fn effective_offset(calibration: Option<&Calibration>, pose: &PoseSettings) -> Pose {
    calibration
        .filter(|c| c.matches(pose))
        .map(|c| c.offset)
        .unwrap_or_else(default_offset)
}

/// パネルの姿勢を求める。ヨーの補正、センサー途絶時の代用、追跡が切れたときの固定も受け持つ
///
/// - コントローラー固定（T3-4）：コントローラーの姿勢×位置合わせのずれ。追跡が切れたら最後の姿勢に
///   止めて薄く表示し、戻れば元に戻す。画面が消えていれば隠す
/// - ハンドトラッキング（T9-3）は未実装で、手の姿勢をコントローラーと同じく扱う
#[derive(Debug, Default)]
pub struct PoseFusion {
    last_pose: Option<Pose>,
}

impl PoseFusion {
    pub fn new() -> Self {
        Self::default()
    }

    /// 最後に求めた姿勢を忘れる（基準や側を変えたとき、SteamVRとつなぎ直したとき）
    pub fn reset(&mut self) {
        self.last_pose = None;
    }

    pub fn update(
        &mut self,
        base: &PoseSample,
        _orientation: Option<&OrientationSample>,
        calibration: &Calibration,
        screen_off: bool,
        _now: Instant,
    ) -> PanelState {
        let mode = PanelMode::Controller;
        let (pose, visibility) = match (base.tracked(), self.last_pose) {
            (Some(p), _) => {
                let pose = p.compose(&calibration.offset);
                self.last_pose = Some(pose);
                (pose, PanelVisibility::Shown)
            }
            (None, Some(last)) => (last, PanelVisibility::Dimmed),
            (None, None) => (Pose::default(), PanelVisibility::Hidden),
        };
        PanelState {
            pose,
            visibility: if screen_off {
                PanelVisibility::Hidden
            } else {
                visibility
            },
            mode,
            screen_off,
            has_pose: self.last_pose.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::PoseSourceKind;

    fn calib(offset: Pose) -> Calibration {
        Calibration {
            source: PoseSourceKind::Controller,
            side: Side::Right,
            offset,
            calibrated_at_ms: 0,
        }
    }

    fn tracking(p: Pose) -> PoseSample {
        PoseSample {
            pose: Some(p),
            state: TrackingState::Tracking,
        }
    }

    const LOST: PoseSample = PoseSample {
        pose: None,
        state: TrackingState::Lost,
    };

    fn controller() -> Pose {
        Pose::new(Vec3::new(0.2, 1.1, -0.3), Quat::from_rotation_y(0.5))
    }

    #[test]
    fn panel_follows_controller_with_calibration_offset() {
        let mut f = PoseFusion::new();
        let offset = Pose::new(Vec3::new(0.0, 0.05, -0.02), Quat::from_rotation_x(0.2));
        let s = f.update(
            &tracking(controller()),
            None,
            &calib(offset),
            false,
            Instant::now(),
        );
        assert_eq!(s.visibility, PanelVisibility::Shown);
        assert_eq!(s.mode, PanelMode::Controller);
        assert!(s.pose.approx_eq(&controller().compose(&offset), 1e-6));
        // コントローラーの位置から、回したずれの分だけ離れる
        let expected_pos = controller().position + controller().rotation * offset.position;
        assert!(s.pose.position.abs_diff_eq(expected_pos, 1e-6));
        assert_eq!(s.chip(), Some(Chip::Tracking));
    }

    #[test]
    fn lost_tracking_freezes_last_pose_dimmed_then_recovers() {
        let mut f = PoseFusion::new();
        let c = calib(default_offset());
        let now = Instant::now();
        let shown = f.update(&tracking(controller()), None, &c, false, now);

        // 追跡が切れた：最後の姿勢のまま薄く表示
        let lost = f.update(&LOST, None, &c, false, now);
        assert_eq!(lost.visibility, PanelVisibility::Dimmed);
        assert_eq!(lost.pose, shown.pose);
        assert_eq!(lost.chip(), Some(Chip::HandLost));
        // 姿勢があっても追跡が切れていれば使わない
        let stale = PoseSample {
            pose: Some(Pose::default()),
            state: TrackingState::Lost,
        };
        assert_eq!(f.update(&stale, None, &c, false, now).pose, shown.pose);

        // 戻れば元に戻る
        let moved = Pose::new(Vec3::new(-0.1, 1.0, 0.0), Quat::IDENTITY);
        let back = f.update(&tracking(moved), None, &c, false, now);
        assert_eq!(back.visibility, PanelVisibility::Shown);
        assert!(back.pose.approx_eq(&moved.compose(&c.offset), 1e-6));
    }

    #[test]
    fn never_tracked_is_hidden_without_chip() {
        let mut f = PoseFusion::new();
        let s = f.update(&LOST, None, &calib(default_offset()), false, Instant::now());
        assert_eq!(s.visibility, PanelVisibility::Hidden);
        assert_eq!(s.chip(), None);
        let off = f.update(&LOST, None, &calib(default_offset()), true, Instant::now());
        assert_eq!(off.chip(), None);
    }

    #[test]
    fn screen_off_hides_and_shows_its_chip() {
        let mut f = PoseFusion::new();
        let c = calib(default_offset());
        let now = Instant::now();
        let off = f.update(&tracking(controller()), None, &c, true, now);
        assert_eq!(off.visibility, PanelVisibility::Hidden);
        assert_eq!(off.chip(), Some(Chip::ScreenOff));
        // 隠している間も姿勢は追う（画面がつけば、その場所に戻る）
        assert!(off.pose.approx_eq(&controller().compose(&c.offset), 1e-6));
        // 追跡が切れていても、画面が消えていれば隠す
        let off_lost = f.update(&LOST, None, &c, true, now);
        assert_eq!(off_lost.visibility, PanelVisibility::Hidden);
        assert_eq!(off_lost.chip(), Some(Chip::ScreenOff));
        // 画面がつけば表示に戻る
        let on = f.update(&tracking(controller()), None, &c, false, now);
        assert_eq!(on.visibility, PanelVisibility::Shown);
    }

    #[test]
    fn reset_forgets_last_pose() {
        let mut f = PoseFusion::new();
        let c = calib(default_offset());
        f.update(&tracking(controller()), None, &c, false, Instant::now());
        f.reset();
        let s = f.update(&LOST, None, &c, false, Instant::now());
        assert_eq!(s.visibility, PanelVisibility::Hidden);
    }

    #[test]
    fn effective_offset_ignores_stale_calibration() {
        let offset = Pose::new(Vec3::new(0.0, 0.1, 0.0), Quat::IDENTITY);
        let c = calib(offset);
        let same = PoseSettings {
            source: PoseSourceKind::Controller,
            side: Side::Right,
        };
        let other = PoseSettings {
            source: PoseSourceKind::Controller,
            side: Side::Left,
        };
        assert_eq!(effective_offset(Some(&c), &same), offset);
        assert_eq!(effective_offset(Some(&c), &other), default_offset());
        assert_eq!(effective_offset(None, &same), default_offset());
    }
}
