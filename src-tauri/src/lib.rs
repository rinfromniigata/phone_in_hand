mod app_state;
mod commands;

use pih_core::settings_store::JsonSettingsStore;
use tauri::{Emitter, Manager};

use crate::app_state::{AppState, Modules};

/// UIへ送るイベントの名前
pub const EVENT_STATE_CHANGED: &str = "state_changed";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let dir = app.path().app_data_dir()?;
            let handle = app.handle().clone();
            let state = AppState::new(
                Box::new(JsonSettingsStore::new(dir)),
                Modules::default(),
                Box::new(move |snapshot| {
                    if let Err(e) = handle.emit(EVENT_STATE_CHANGED, snapshot) {
                        log::warn!("state_changed を送れません: {e}");
                    }
                }),
            )?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_settings,
            commands::update_settings,
            commands::set_display_enabled,
            commands::list_capture_devices,
            commands::list_windows,
            commands::start_video,
            commands::adb_pair,
            commands::adb_connect,
            commands::adb_install_sensor_app,
            commands::sensor_qr_payload,
            commands::start_calibration,
            commands::finish_calibration,
            commands::cancel_calibration,
            commands::complete_setup,
            commands::open_licenses,
        ])
        .run(tauri::generate_context!())
        .expect("Tauriのアプリを起動できませんでした");
}
