//! `controller_pose_source`（T3-3）：左右のコントローラーの姿勢と追跡状態（SteamVRのstanding空間）。
//!
//! 姿勢は次に表示されるときの予測の値で、姿勢のスレッドがSteamVRのフレームごとに読む。
use std::sync::Arc;

use pih_core::geometry::Side;
use pih_core::pose::{PoseSample, PoseSource, TrackingState};

use crate::openvr::OpenVr;

pub struct ControllerPoseSource {
    vr: Arc<OpenVr>,
}

impl ControllerPoseSource {
    pub fn new(vr: Arc<OpenVr>) -> Self {
        ControllerPoseSource { vr }
    }
}

impl PoseSource for ControllerPoseSource {
    fn sample(&mut self, side: Side) -> PoseSample {
        match self.vr.controller_pose(side) {
            Some(pose) => PoseSample {
                pose: Some(pose),
                state: TrackingState::Tracking,
            },
            // つながっていない、コントローラーがない、追跡が切れている
            None => PoseSample {
                pose: None,
                state: TrackingState::Lost,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn without_steamvr_reports_lost() {
        let vr = Arc::new(OpenVr::new(PathBuf::from("Z:/no/openvr_api.dll")));
        let mut src = ControllerPoseSource::new(vr);
        for side in [Side::Left, Side::Right] {
            let s = src.sample(side);
            assert_eq!(s.state, TrackingState::Lost);
            assert!(s.pose.is_none());
        }
    }
}
