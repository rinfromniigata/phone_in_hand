//! 各モジュールの状態をまとめた `AppSnapshot`（`app_state` がUIへ `state_changed` で送る）
//!
//! UIは、状態の列挙をそのまま辞書のキーに対応させて表示する（UIで文を組み立てない）。
use serde::Serialize;

use crate::geometry::Side;
use crate::pose::PanelMode;
use crate::settings::{Calibration, PhoneKind, PoseSourceKind, Settings};
use crate::video::VideoStatus;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppSnapshot {
    /// ホームの「VRに表示」
    pub display_enabled: bool,
    pub video: VideoStatus,
    pub sensor: SensorStatus,
    pub adb: AdbStatus,
    pub tracking: TrackingStatus,
    pub vrchat: VrchatLink,
    pub calibration: CalibrationStatus,
}

/// スマホのセンサー（ハンドトラッキングのときだけ使う）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SensorStatus {
    Unavailable,
    /// コントローラーに固定しているため使わない
    NotUsed,
    Receiving,
    /// 値が届かなくなってからの秒数
    Lost {
        seconds: u32,
    },
}

/// ワイヤレスデバッグ（Androidのみ）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AdbStatus {
    Unavailable,
    /// iPhoneでは使わない
    NotUsed,
    Disconnected,
    Connected {
        device: String,
        show_touches: bool,
    },
}

/// 位置の追跡
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TrackingStatus {
    Unavailable,
    Tracking { mode: PanelMode, side: Side },
    Lost { side: Side },
}

/// VRChat（OSC）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VrchatLink {
    Unavailable,
    /// VRChatが起動していないか、OSCが無効
    NotRunning,
    Connected {
        params_found: bool,
    },
}

/// 位置合わせの結果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    Missing,
    Valid,
    /// 基準や側を変えたため、合わせ直しが要る
    Stale,
}

impl CalibrationStatus {
    pub fn of(calibration: Option<&Calibration>, settings: &Settings) -> Self {
        match calibration {
            None => CalibrationStatus::Missing,
            Some(c) if c.matches(&settings.pose) => CalibrationStatus::Valid,
            Some(_) => CalibrationStatus::Stale,
        }
    }
}

impl AppSnapshot {
    /// 機器のモジュールがまだ何も報告していないときの状態。
    /// 設定から決まるもの（使わない機能、位置合わせ）だけを反映する
    pub fn initial(settings: &Settings, calibration: Option<&Calibration>) -> Self {
        let sensor = match settings.pose.source {
            PoseSourceKind::Controller => SensorStatus::NotUsed,
            PoseSourceKind::Hand => SensorStatus::Unavailable,
        };
        let adb = match settings.phone_kind {
            PhoneKind::Iphone => AdbStatus::NotUsed,
            PhoneKind::Android => AdbStatus::Unavailable,
        };
        AppSnapshot {
            display_enabled: settings.display.enabled,
            video: VideoStatus::Unavailable,
            sensor,
            adb,
            tracking: TrackingStatus::Unavailable,
            vrchat: VrchatLink::Unavailable,
            calibration: CalibrationStatus::of(calibration, settings),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Pose;
    use serde_json::json;

    #[test]
    fn initial_reflects_settings() {
        let mut s = Settings::default();
        let snap = AppSnapshot::initial(&s, None);
        assert_eq!(snap.sensor, SensorStatus::NotUsed);
        assert_eq!(snap.adb, AdbStatus::Unavailable);
        assert_eq!(snap.calibration, CalibrationStatus::Missing);

        s.phone_kind = PhoneKind::Iphone;
        s.pose.source = PoseSourceKind::Hand;
        let c = Calibration {
            source: PoseSourceKind::Controller,
            side: Side::Right,
            offset: Pose::default(),
            calibrated_at_ms: 0,
        };
        let snap = AppSnapshot::initial(&s, Some(&c));
        assert_eq!(snap.adb, AdbStatus::NotUsed);
        assert_eq!(snap.sensor, SensorStatus::Unavailable);
        assert_eq!(snap.calibration, CalibrationStatus::Stale);
    }

    #[test]
    fn serializes_with_state_tags() {
        let snap = AppSnapshot {
            display_enabled: true,
            video: VideoStatus::NoSignal,
            sensor: SensorStatus::Lost { seconds: 3 },
            adb: AdbStatus::Connected {
                device: "Pixel".into(),
                show_touches: true,
            },
            tracking: TrackingStatus::Tracking {
                mode: PanelMode::HandFallback,
                side: Side::Left,
            },
            vrchat: VrchatLink::Connected {
                params_found: false,
            },
            calibration: CalibrationStatus::Valid,
        };
        assert_eq!(
            serde_json::to_value(&snap).unwrap(),
            json!({
                "display_enabled": true,
                "video": { "state": "no_signal" },
                "sensor": { "state": "lost", "seconds": 3 },
                "adb": { "state": "connected", "device": "Pixel", "show_touches": true },
                "tracking": { "state": "tracking", "mode": "hand_fallback", "side": "left" },
                "vrchat": { "state": "connected", "params_found": false },
                "calibration": "valid"
            })
        );
    }
}
