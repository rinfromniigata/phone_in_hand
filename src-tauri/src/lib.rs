mod app_state;
mod commands;
mod controller_pose;
mod gpu;
mod openvr;
mod overlay;
mod runtime;
mod startup;
mod window_capture;

use std::path::PathBuf;
use std::sync::Arc;

use pih_core::settings_store::JsonSettingsStore;
use tauri::{Emitter, Manager, RunEvent};

use crate::app_state::{AppState, Modules};
use crate::openvr::OpenVr;
use crate::runtime::{EventSink, PreviewSink, Runtime};

/// UIへ送るイベントの名前
pub const EVENT_STATE_CHANGED: &str = "state_changed";
pub const EVENT_PREVIEW_FRAME: &str = "preview_frame";
pub const EVENT_CALIBRATION_PROGRESS: &str = "calibration_progress";

/// 同梱のリソース（openvr_api.dll、位置合わせの入力の定義）の場所。
/// インストールした版も `tauri dev` も、リソースのフォルダ（実行ファイルと同じ場所）にある
fn resource(app: &tauri::App, path: &str) -> PathBuf {
    app.path()
        .resource_dir()
        .map(|d| d.join(path))
        .unwrap_or_else(|_| PathBuf::from(path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
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
            let progress_handle = app.handle().clone();
            let state = AppState::new(
                Box::new(JsonSettingsStore::new(dir.clone())),
                Modules::default(),
                Box::new(move |snapshot| {
                    if let Err(e) = handle.emit(EVENT_STATE_CHANGED, snapshot) {
                        log::warn!("state_changed を送れません: {e}");
                    }
                }),
                Box::new(move |progress| {
                    let _ = progress_handle.emit(EVENT_CALIBRATION_PROGRESS, progress);
                }),
            )?;
            let settings = state.settings();
            app.manage(state);

            // 起動時にウィンドウを最小化する（S03c）
            if settings.general.start_minimized
                && let Some(w) = app.get_webview_window("main")
                && let Err(e) = w.minimize()
            {
                log::warn!("ウィンドウを最小化できません: {e}");
            }

            // 映像と姿勢のスレッド。知らせは AppState へ、プレビューはUIへ直接送る
            let handle = app.handle().clone();
            let events: EventSink = Arc::new(move |ev| {
                if let Some(s) = handle.try_state::<AppState>() {
                    s.on_runtime_event(ev);
                }
            });
            let handle = app.handle().clone();
            let preview: PreviewSink = Arc::new(move |frame| {
                let _ = handle.emit(EVENT_PREVIEW_FRAME, frame);
            });
            let vr = Arc::new(OpenVr::new(resource(app, "openvr_api.dll")));
            let runtime = Arc::new(Runtime::start(
                vr,
                events,
                preview,
                &settings,
                None,
                resource(app, "openvr/actions.json"),
            ));
            // SteamVRと一緒に起動するためのマニフェスト（今の実行ファイルの場所で書き直す）
            match std::env::current_exe().and_then(|exe| startup::write_manifest(&dir, &exe)) {
                Ok(path) => runtime.set_app_manifest(path),
                Err(e) => log::warn!("SteamVRのマニフェストを書けません: {e}"),
            }
            app.manage(runtime.clone());
            app.state::<AppState>().attach_runtime(Box::new(runtime));
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
            commands::set_overlay_images,
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
        .build(tauri::generate_context!())
        .expect("Tauriのアプリを起動できませんでした");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // スレッドを止め、SteamVRとの接続を閉じる
            if let Some(rt) = app.try_state::<Arc<Runtime>>() {
                rt.shutdown();
            }
        }
    });
}
