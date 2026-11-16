//! 位置合わせ（S01e。imple 2章「位置合わせ」）の手順。
//!
//! VRの中に画面の見本を、スマホを付けたコントローラーに対する今のずれで出す。利用者は、もう一方の
//! コントローラーのグリップで見本をつかんで動かし・回し、実物のスマホに重ねて、トリガーで決定する。
//! 決定した時点の「コントローラーの姿勢に対する見本の相対姿勢」が、位置合わせの結果（ずれ）になる。
use serde::Serialize;

use crate::geometry::Pose;
use crate::settings::PoseSettings;

/// 1フレーム分の入力
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CalibrationInput {
    /// スマホを付けたコントローラー（追跡中のときだけ）
    pub phone: Option<Pose>,
    /// もう一方のコントローラー（追跡中のときだけ）
    pub other: Option<Pose>,
    /// もう一方のコントローラーのグリップ（つかむ）
    pub grab: bool,
    /// どちらかのトリガー（決定）
    pub confirm: bool,
}

/// 1フレーム分の結果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalibrationStep {
    /// 続ける。見本を出す姿勢（まだ一度もコントローラーの姿勢がなければ `None`）
    Continue { sample: Option<Pose> },
    /// 決定した。コントローラーに対する見本のずれ
    Finished { offset: Pose },
}

/// UIへ知らせる進み具合（`calibration_progress` イベント）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CalibrationProgress {
    Running,
    Finished,
    Cancelled,
    /// SteamVRとつながっていない、または途中で切れた
    Failed,
}

#[derive(Debug, Clone)]
pub struct CalibrationSession {
    pose: PoseSettings,
    /// スマホを付けたコントローラーに対する、見本のずれ
    offset: Pose,
    /// つかんでいる間の、もう一方のコントローラーに対する見本の相対姿勢
    held: Option<Pose>,
    /// 前のフレームでトリガーを押していたか（押した瞬間だけを決定にする）
    confirm_was_down: bool,
    last_sample: Option<Pose>,
}

impl CalibrationSession {
    /// `start_offset` は今のずれ（前の結果か既定のずれ）
    pub fn new(pose: PoseSettings, start_offset: Pose) -> Self {
        CalibrationSession {
            pose,
            offset: start_offset,
            held: None,
            // 始めた時点で押していたトリガーは、いったん離すまで決定にしない
            confirm_was_down: true,
            last_sample: None,
        }
    }

    pub fn pose_settings(&self) -> PoseSettings {
        self.pose
    }

    /// 今のずれ（PCから決定するときに使う）
    pub fn offset(&self) -> Pose {
        self.offset
    }

    pub fn update(&mut self, input: &CalibrationInput) -> CalibrationStep {
        let confirm_pressed = input.confirm && !self.confirm_was_down;
        self.confirm_was_down = input.confirm;

        let Some(phone) = input.phone else {
            // スマホ側の追跡が切れている間は、見本を最後の場所に止め、つかみ直しもしない
            self.held = None;
            return CalibrationStep::Continue {
                sample: self.last_sample,
            };
        };

        let sample = match (input.grab, input.other) {
            (true, Some(other)) => {
                let held = *self
                    .held
                    .get_or_insert_with(|| other.relative(&phone.compose(&self.offset)));
                let sample = other.compose(&held);
                // 放したときにその場に残るよう、スマホ側に対するずれも追う
                self.offset = phone.relative(&sample);
                sample
            }
            _ => {
                self.held = None;
                phone.compose(&self.offset)
            }
        };
        self.last_sample = Some(sample);

        if confirm_pressed && self.held.is_none() {
            return CalibrationStep::Finished {
                offset: self.offset,
            };
        }
        CalibrationStep::Continue {
            sample: Some(sample),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Quat, Side, Vec3};
    use crate::pose::default_offset;
    use crate::settings::PoseSourceKind;

    fn session() -> CalibrationSession {
        CalibrationSession::new(
            PoseSettings {
                source: PoseSourceKind::Controller,
                side: Side::Right,
            },
            default_offset(),
        )
    }

    fn phone() -> Pose {
        Pose::new(Vec3::new(0.3, 1.0, -0.2), Quat::from_rotation_y(0.4))
    }

    fn input(other: Pose, grab: bool, confirm: bool) -> CalibrationInput {
        CalibrationInput {
            phone: Some(phone()),
            other: Some(other),
            grab,
            confirm,
        }
    }

    fn sample_of(step: CalibrationStep) -> Pose {
        match step {
            CalibrationStep::Continue { sample: Some(s) } => s,
            other => panic!("見本の姿勢がない: {other:?}"),
        }
    }

    #[test]
    fn sample_starts_at_current_offset() {
        let mut s = session();
        let hand = Pose::new(Vec3::new(-0.2, 1.0, -0.2), Quat::IDENTITY);
        let shown = sample_of(s.update(&input(hand, false, false)));
        assert!(shown.approx_eq(&phone().compose(&default_offset()), 1e-5));
    }

    #[test]
    fn grabbing_moves_the_sample_and_release_keeps_it() {
        let mut s = session();
        let hand = Pose::new(Vec3::new(-0.2, 1.0, -0.2), Quat::IDENTITY);
        let before = sample_of(s.update(&input(hand, false, false)));

        // つかんで、もう一方の手を10cm上げて少し回す
        sample_of(s.update(&input(hand, true, false)));
        let moved_hand = Pose::new(
            hand.position + Vec3::new(0.0, 0.1, 0.0),
            Quat::from_rotation_z(0.2),
        );
        let moved = sample_of(s.update(&input(moved_hand, true, false)));
        let expected = moved_hand.compose(&hand.relative(&before));
        assert!(moved.approx_eq(&expected, 1e-5));

        // 放すと、スマホ側に付いたままその場に残る
        let released = sample_of(s.update(&input(moved_hand, false, false)));
        assert!(released.approx_eq(&moved, 1e-5));
        assert!(s.offset().approx_eq(&phone().relative(&moved), 1e-5));
    }

    #[test]
    fn trigger_finishes_with_current_offset() {
        let mut s = session();
        let hand = Pose::default();
        // 始めた時点で押していたトリガーは、離すまで決定にしない
        assert!(matches!(
            s.update(&input(hand, false, true)),
            CalibrationStep::Continue { .. }
        ));
        s.update(&input(hand, false, false));
        match s.update(&input(hand, false, true)) {
            CalibrationStep::Finished { offset } => {
                assert!(offset.approx_eq(&default_offset(), 1e-5))
            }
            other => panic!("決定しない: {other:?}"),
        }
    }

    #[test]
    fn trigger_while_grabbing_is_ignored() {
        let mut s = session();
        let hand = Pose::default();
        s.update(&input(hand, false, false));
        assert!(matches!(
            s.update(&input(hand, true, true)),
            CalibrationStep::Continue { .. }
        ));
    }

    #[test]
    fn lost_phone_keeps_last_sample() {
        let mut s = session();
        let hand = Pose::default();
        let shown = sample_of(s.update(&input(hand, false, false)));
        let lost = CalibrationInput {
            phone: None,
            other: Some(hand),
            grab: true,
            confirm: false,
        };
        assert_eq!(
            s.update(&lost),
            CalibrationStep::Continue {
                sample: Some(shown)
            }
        );
        // 最初から追跡が切れていれば、見本は出さない
        let mut fresh = session();
        assert_eq!(
            fresh.update(&lost),
            CalibrationStep::Continue { sample: None }
        );
    }
}
