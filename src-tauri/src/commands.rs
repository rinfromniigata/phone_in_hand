//! Tauriのコマンド（UIとの境界。imple 2章）
//!
//! 失敗はUIへ `{ "code": "unavailable" }` のような識別子だけで返し、文言はUIの辞書で引く。
use pih_core::adb::{AdbTarget, DeviceInfo};
use pih_core::error::ErrorCode;
use pih_core::settings::{PoseSettings, Settings, VideoSettings};
use pih_core::snapshot::AppSnapshot;
use pih_core::video::{CaptureDevice, WindowInfo};
use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;
use crate::overlay::OverlayImages;

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
pub fn list_capture_devices() -> CmdResult<Vec<CaptureDevice>> {
    // T4-1で実装する
    unavailable()
}

#[tauri::command]
pub fn list_windows(state: State<'_, AppState>) -> CmdResult<Vec<WindowInfo>> {
    Ok(state.list_windows()?)
}

/// 保存していない映像の設定で取り込む（映像のページのプレビュー）
#[tauri::command]
pub fn start_video(state: State<'_, AppState>, video: VideoSettings) -> CmdResult<()> {
    Ok(state.start_video(&video)?)
}

/// UIが描いたVR内のチップと位置合わせの見本の画像
#[tauri::command]
pub fn set_overlay_images(state: State<'_, AppState>, images: OverlayImages) -> CmdResult<()> {
    Ok(state.set_overlay_images(images)?)
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

/// 位置合わせを始める（S01e）。SteamVRとつながっていなければ `unavailable`
#[tauri::command]
pub fn start_calibration(state: State<'_, AppState>, pose: PoseSettings) -> CmdResult<()> {
    Ok(state.start_calibration(pose)?)
}

/// 位置合わせを、今の見本の位置で決定する
#[tauri::command]
pub fn finish_calibration(state: State<'_, AppState>) -> CmdResult<()> {
    Ok(state.finish_calibration()?)
}

/// 位置合わせをやめる。前の結果は残す
#[tauri::command]
pub fn cancel_calibration(state: State<'_, AppState>) {
    state.cancel_calibration();
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
