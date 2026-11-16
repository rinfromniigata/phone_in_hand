//! 保存するデータのモデル（imple 7章）。`settings.json`、`calibration.json`、`restore_state.json`
use serde::{Deserialize, Serialize};

use crate::geometry::{Pose, Rect, Side};
use crate::video::VideoSourceKind;

/// `settings.json` のスキーマの版。形を変えたら上げ、[`crate::settings_store`] で移行する
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

/// センサー送信アプリからのUDPを待ち受けるポートの既定値
pub const DEFAULT_SENSOR_PORT: u16 = 39570;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub setup_completed: bool,
    pub phone_kind: PhoneKind,
    pub video: VideoSettings,
    pub pose: PoseSettings,
    pub display: DisplaySettings,
    pub input: InputSettings,
    pub vrchat: VrchatSettings,
    pub general: GeneralSettings,
    pub sensor: SensorSettings,
    pub adb_devices: Vec<AdbDevice>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: SETTINGS_SCHEMA_VERSION,
            setup_completed: false,
            phone_kind: PhoneKind::default(),
            video: VideoSettings::default(),
            pose: PoseSettings::default(),
            display: DisplaySettings::default(),
            input: InputSettings::default(),
            vrchat: VrchatSettings::default(),
            general: GeneralSettings::default(),
            sensor: SensorSettings::default(),
            adb_devices: Vec::new(),
        }
    }
}

impl Settings {
    /// 値の範囲を確かめる（UIからの変更を保存する前に呼ぶ）
    pub fn validate(&self) -> crate::Result<()> {
        let invalid = |what: &str| Err(crate::Error::Invalid(what.to_owned()));
        if self.schema_version != SETTINGS_SCHEMA_VERSION {
            return invalid("schema_version");
        }
        if !(self.display.size_inch.is_finite() && self.display.size_inch > 0.0) {
            return invalid("display.size_inch");
        }
        if !(0.0..=1.0).contains(&self.display.brightness) {
            return invalid("display.brightness");
        }
        if self.vrchat.send_port == 0 || self.vrchat.receive_port == 0 || self.sensor.port == 0 {
            return invalid("port");
        }
        if self.vrchat.param_visible.trim().is_empty()
            || self.vrchat.param_is_left.trim().is_empty()
        {
            return invalid("vrchat.param");
        }
        Ok(())
    }

    /// 部分的な変更（JSON Merge Patch）を当てた新しい設定を返す。形や値が正しくなければ失敗する
    pub fn patched(&self, patch: &serde_json::Value) -> crate::Result<Settings> {
        let mut v = serde_json::to_value(self)?;
        merge_patch(&mut v, patch);
        let next: Settings = serde_json::from_value(v)?;
        next.validate()?;
        Ok(next)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhoneKind {
    #[default]
    Android,
    Iphone,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoSettings {
    pub source: VideoSourceKind,
    pub hdmi: HdmiSettings,
    pub mirror: MirrorSettings,
    pub window: WindowSettings,
    pub crop: CropSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HdmiSettings {
    pub device_id: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorSettings {
    pub connection: MirrorConnection,
    pub codec: MirrorCodec,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorConnection {
    #[default]
    Usb,
    Wifi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorCodec {
    /// H.265を優先する
    #[default]
    Auto,
    H264,
    H265,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    pub title: Option<String>,
    pub executable: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CropSettings {
    pub mode: CropMode,
    /// `mode` が `manual` のときの矩形
    pub rect: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropMode {
    #[default]
    Auto,
    Manual,
}

/// 位置の基準
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoseSourceKind {
    #[default]
    Controller,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PoseSettings {
    pub source: PoseSourceKind,
    pub side: Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplaySettings {
    pub enabled: bool,
    /// パネルの対角の大きさ（インチ）
    pub size_inch: f32,
    /// 0〜1
    pub brightness: f32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            enabled: false,
            size_inch: 6.7,
            brightness: 0.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputSettings {
    pub show_touches: bool,
    pub keep_awake: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        InputSettings {
            show_touches: true,
            keep_awake: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VrchatSettings {
    pub send_port: u16,
    pub receive_port: u16,
    pub param_visible: String,
    pub param_is_left: String,
}

impl Default for VrchatSettings {
    fn default() -> Self {
        VrchatSettings {
            send_port: 9000,
            receive_port: 9001,
            param_visible: "PhoneInHand/Visible".to_owned(),
            param_is_left: "PhoneInHand/IsLeft".to_owned(),
        }
    }
}

/// 表示の言語。`auto` はOSのロケールに合わせる（ja・en・zh-Hans以外はja）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-Hans")]
    ZhHans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub language: Language,
    pub theme: Theme,
    pub start_with_steamvr: bool,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSettings {
    /// 接続トークン。ハンドトラッキングのセットアップで作る
    pub token: Option<String>,
    pub port: u16,
}

impl Default for SensorSettings {
    fn default() -> Self {
        SensorSettings {
            token: None,
            port: DEFAULT_SENSOR_PORT,
        }
    }
}

/// ペア設定した端末。同じシリアルは1件にまとめる
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdbDevice {
    pub serial: String,
    pub name: String,
    pub last_address: Option<String>,
}

/// `calibration.json`：位置合わせの結果
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub source: PoseSourceKind,
    pub side: Side,
    /// 基準の姿勢に対するパネルの相対姿勢（位置のずれと回転のずれ）
    pub offset: Pose,
    /// 位置合わせを終えた日時（UNIX時刻、ミリ秒）
    pub calibrated_at_ms: u64,
}

impl Calibration {
    /// 基準や側を変えた後は、前の結果は使えない
    pub fn matches(&self, pose: &PoseSettings) -> bool {
        self.source == pose.source && self.side == pose.side
    }
}

/// `restore_state.json`：ADBで変える前の端末の設定値（変更中だけ存在する）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreState {
    pub serial: String,
    /// 変更前の「タップを表示」の値
    pub show_touches: Option<String>,
    /// 変更前の画面を消さない設定の値
    pub stay_on_while_plugged_in: Option<String>,
}

/// 保存したもの全部
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stored {
    pub settings: Settings,
    pub calibration: Option<Calibration>,
    pub restore_state: Option<RestoreState>,
    /// 読めずに `*.broken` へ退避したファイルの名前
    pub recovered: Vec<&'static str>,
}

/// RFC 7386（JSON Merge Patch）を適用する
pub fn merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    use serde_json::Value;
    let Value::Object(p) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(serde_json::Map::new());
    }
    if let Value::Object(t) = target {
        for (k, v) in p {
            if v.is_null() {
                t.remove(k);
            } else {
                merge_patch(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_valid() {
        Settings::default().validate().unwrap();
    }

    /// UIがTauriの外（ブラウザでの開発）で使う既定値を、Rustの既定値と一致させる。
    /// 食い違ったら `UPDATE_FIXTURES=1 cargo test -p pih_core` で書き直す
    #[test]
    fn ui_default_settings_fixture_is_current() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/lib/state/default_settings.json");
        let expected = serde_json::to_string_pretty(&Settings::default()).unwrap() + "\n";
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(&path, &expected).unwrap();
        }
        let actual = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert_eq!(
            actual, expected,
            "src/lib/state/default_settings.json が古い。UPDATE_FIXTURES=1 cargo test -p pih_core で書き直す"
        );
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings =
            serde_json::from_value(json!({ "schema_version": 1, "display": { "enabled": true } }))
                .unwrap();
        assert!(s.display.enabled);
        assert_eq!(s.display.size_inch, DisplaySettings::default().size_inch);
        assert_eq!(s.vrchat, VrchatSettings::default());
    }

    #[test]
    fn patch_changes_only_given_fields() {
        let base = Settings::default();
        let next = base
            .patched(&json!({ "display": { "brightness": 0.5 } }))
            .unwrap();
        assert_eq!(next.display.brightness, 0.5);
        assert_eq!(next.display.size_inch, base.display.size_inch);
        assert_eq!(next.video, base.video);
    }

    #[test]
    fn switching_video_source_keeps_other_sources() {
        let mut base = Settings::default();
        base.video.hdmi.device_id = Some("cap0".into());
        let next = base
            .patched(&json!({ "video": { "source": "window" } }))
            .unwrap();
        assert_eq!(next.video.source, VideoSourceKind::Window);
        assert_eq!(next.video.hdmi.device_id.as_deref(), Some("cap0"));
    }

    #[test]
    fn patch_rejects_out_of_range_and_wrong_types() {
        let base = Settings::default();
        assert!(
            base.patched(&json!({ "display": { "brightness": 2.0 } }))
                .is_err()
        );
        assert!(
            base.patched(&json!({ "vrchat": { "send_port": 0 } }))
                .is_err()
        );
        assert!(
            base.patched(&json!({ "general": { "language": "fr" } }))
                .is_err()
        );
        assert!(
            base.patched(&json!({ "display": { "enabled": "yes" } }))
                .is_err()
        );
    }

    #[test]
    fn language_uses_bcp47_tags() {
        let s = Settings::default()
            .patched(&json!({ "general": { "language": "zh-Hans" } }))
            .unwrap();
        assert_eq!(s.general.language, Language::ZhHans);
        assert_eq!(
            serde_json::to_value(s.general.language).unwrap(),
            json!("zh-Hans")
        );
    }

    #[test]
    fn calibration_matches_only_same_source_and_side() {
        let c = Calibration {
            source: PoseSourceKind::Controller,
            side: Side::Right,
            offset: Pose::default(),
            calibrated_at_ms: 0,
        };
        let same = PoseSettings {
            source: PoseSourceKind::Controller,
            side: Side::Right,
        };
        let other_side = PoseSettings {
            source: PoseSourceKind::Controller,
            side: Side::Left,
        };
        let other_source = PoseSettings {
            source: PoseSourceKind::Hand,
            side: Side::Right,
        };
        assert!(c.matches(&same));
        assert!(!c.matches(&other_side));
        assert!(!c.matches(&other_source));
    }
}
