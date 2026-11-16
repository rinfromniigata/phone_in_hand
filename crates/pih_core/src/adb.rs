//! ADB（Androidのみ。`adb_controller`）
use std::net::SocketAddr;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;

/// 接続先
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdbTarget {
    Usb { serial: String },
    Wifi { host: String, port: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub name: String,
}

/// VRChatとつながっている間だけ変える、端末の設定
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSettings {
    /// 「タップを表示」
    pub show_touches: bool,
    /// 画面を消さない
    pub keep_awake: bool,
}

/// scrcpy-serverの起動の設定
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorOptions {
    pub codec: crate::settings::MirrorCodec,
}

/// scrcpy-serverの映像のソケット（T7-2で中身を決める）
#[derive(Debug)]
pub struct MirrorStream {
    pub video: std::net::TcpStream,
}

/// センサー送信アプリへ渡す接続情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorConfig {
    pub address: SocketAddr,
    pub token: String,
}

pub trait AdbController: Send {
    fn pair(&mut self, host: &str, port: u16, code: &str) -> Result<()>;
    /// USBかWi-Fi
    fn connect(&mut self, target: &AdbTarget) -> Result<DeviceInfo>;
    fn install(&mut self, apk: &Path) -> Result<()>;
    /// 「タップを表示」と画面を消さない設定。変更前の値を `restore_state` に記録してから変える
    fn apply_session_settings(&mut self, want: SessionSettings) -> Result<()>;
    /// 記録した値に戻し、記録を消す
    fn restore(&mut self) -> Result<()>;
    /// scrcpy-serverの送信と起動
    fn start_mirror(&mut self, opts: &MirrorOptions) -> Result<MirrorStream>;
    fn send_sensor_config(&mut self, cfg: &SensorConfig) -> Result<()>;
}
