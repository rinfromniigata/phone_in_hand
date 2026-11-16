//! Tauriのコマンド（UIとの境界。imple 2章）
//!
//! 失敗はUIへ `{ "code": "unavailable" }` のような識別子だけで返し、文言はUIの辞書で引く。
use pih_core::adb::{AdbTarget, DeviceInfo};
use pih_core::error::ErrorCode;
use pih_core::settings::Settings;
use pih_core::snapshot::AppSnapshot;
use pih_core::video::{CaptureDevice, VideoSourceKind, WindowInfo};
use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: ErrorCode,
}

impl From<pih_core::Error> for CommandError {
    fn from(e: pih_core::Error) -> Self {
        log::debug!("コマンドの失敗: {e}");
        CommandError { code: e.code() }
    }
}

type CmdResult<T> = Result<T, CommandError>;

fn unavailable<T>() -> CmdResult<T> {
    Err(pih_core::Error::Unavailable.into())
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> AppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> CmdResult<Settings> {
    Ok(state.update_settings(&patch)?)
}

#[tauri::command]
pub fn set_display_enabled(state: State<'_, AppState>, enabled: bool) -> CmdResult<AppSnapshot> {
    Ok(state.set_display_enabled(enabled)?)
}

#[tauri::command]
pub fn list_capture_devices(state: State<'_, AppState>) -> CmdResult<Vec<CaptureDevice>> {
    Ok(state.list_capture_devices()?)
}

#[tauri::command]
pub fn list_windows(state: State<'_, AppState>) -> CmdResult<Vec<WindowInfo>> {
    Ok(state.list_windows()?)
}

#[tauri::command]
pub fn start_video(state: State<'_, AppState>, source: VideoSourceKind) -> CmdResult<()> {
    Ok(state.start_video(source)?)
}

#[tauri::command]
pub fn adb_pair(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    code: String,
) -> CmdResult<()> {
    Ok(state.adb_pair(&host, port, &code)?)
}

#[tauri::command]
pub fn adb_connect(state: State<'_, AppState>, target: AdbTarget) -> CmdResult<DeviceInfo> {
    Ok(state.adb_connect(&target)?)
}

/// T9-4で実装する
#[tauri::command]
pub fn adb_install_sensor_app() -> CmdResult<()> {
    unavailable()
}

/// T9-4で実装する
#[tauri::command]
pub fn sensor_qr_payload() -> CmdResult<String> {
    unavailable()
}

/// T3-8で実装する
#[tauri::command]
pub fn start_calibration() -> CmdResult<()> {
    unavailable()
}

/// T3-8で実装する
#[tauri::command]
pub fn finish_calibration() -> CmdResult<()> {
    unavailable()
}

/// T3-8で実装する
#[tauri::command]
pub fn cancel_calibration() -> CmdResult<()> {
    unavailable()
}

#[tauri::command]
pub fn complete_setup(state: State<'_, AppState>, patch: serde_json::Value) -> CmdResult<Settings> {
    Ok(state.complete_setup(&patch)?)
}

/// T2-2で実装する（ライセンスの一覧）
#[tauri::command]
pub fn open_licenses() -> CmdResult<()> {
    unavailable()
}
