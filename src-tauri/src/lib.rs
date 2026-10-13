mod commands;
mod optimizer;
mod state;

use state::AppState;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new().expect("failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                let emit_handle = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if commands::close_allowed() {
                            return;
                        }
                        api.prevent_close();
                        let _ = emit_handle.emit("app-close-requested", ());
                    }
                });
            }
            std::thread::spawn(move || {
                if let Some(state) = handle.try_state::<AppState>() {
                    commands::reconcile_quiet(&state);
                }
                commands::mark_startup_sync_done();
                let _ = handle.emit("sync-finished", ());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_setup,
            commands::run_setup,
            commands::get_device_guides,
            commands::get_connection_guide,
            commands::set_custom_adb_path,
            commands::get_adb_status,
            commands::list_devices,
            commands::sync_pull,
            commands::push_reports,
            commands::sync_push,
            commands::scan_packages,
            commands::bulk_uninstall,
            commands::set_package_marks,
            commands::set_airplane_mode,
            commands::get_sync_status,
            commands::get_sync_settings,
            commands::set_sync_provider,
            commands::set_sync_folder,
            commands::set_supabase_config,
            commands::sync_now,
            commands::allow_app_close,
            commands::export_report,
            commands::clear_metadata_cache,
            commands::scan_storage,
            commands::clean_storage,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| ());
}
