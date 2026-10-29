mod active_device;
mod adware;
mod backup;
mod battery_catalog_sync;
mod commands;
mod gphotos;
mod migration;
mod mirror;

use active_device::ActiveDevice;
use adware::state::AppState as AdwareState;
use backup::commands::AppState as BackupState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::Semaphore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    adware::dev_log::init();
    if adware::dev_log::ENABLED {
        adb_bridge::set_dev_logger(adware::dev_log::record);
    }
    migration::import_legacy();
    migration::import_gfm();
    let adware_state = AdwareState::new().expect("failed to initialize adware state");
    let backup_state = BackupState {
        controller: autobackup_core::BackupController::new(),
        scanned_files: Mutex::new(vec![]),
        preview_cache: Mutex::new(HashMap::new()),
        preview_bytes_cache: Mutex::new(HashMap::new()),
        preview_prefix_cache: Mutex::new(HashMap::new()),
        android_preview_semaphore: Arc::new(Semaphore::new(4)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(adware_state)
        .manage(backup_state)
        .manage(ActiveDevice::default())
        .manage(gphotos::GphotosState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_adb_status,
            commands::list_devices,
            commands::load_device_summary,
            commands::reboot_device,
            commands::shutdown_device,
            commands::start_mirror_preview,
            commands::stop_mirror_preview,
            commands::mirror_tap,
            commands::update_battery_catalog,
            active_device::set_active_device,
            active_device::get_active_device,
            adware::commands::allow_app_close,
            adware::commands::dev_log_enabled,
            adware::commands::check_setup,
            adware::commands::run_setup,
            adware::commands::get_device_guides,
            adware::commands::get_connection_guide,
            adware::commands::set_custom_adb_path,
            adware::commands::adware_get_adb_status,
            adware::commands::sync_pull,
            adware::commands::scan_packages,
            adware::commands::bulk_uninstall,
            adware::commands::restore_packages,
            adware::commands::check_browsers,
            adware::commands::clear_browsing_data,
            adware::commands::open_notification_settings,
            adware::commands::open_default_apps,
            adware::commands::set_browser_notifications,
            adware::commands::set_package_marks,
            adware::commands::get_sync_status,
            adware::commands::startup_db_phase,
            adware::commands::get_sync_settings,
            adware::commands::set_sync_provider,
            adware::commands::set_sync_folder,
            adware::commands::set_supabase_config,
            adware::commands::sync_now,
            adware::commands::export_report,
            adware::commands::scan_storage,
            adware::commands::clean_storage,
            backup::commands::get_categories,
            backup::commands::get_default_deselected_extensions,
            backup::commands::get_default_exclusions,
            backup::commands::get_default_exceptions,
            backup::commands::list_removable_drives,
            backup::commands::get_drive_for_path,
            backup::commands::check_adb,
            backup::commands::list_android_devices,
            backup::commands::pause_backup,
            backup::commands::resume_backup,
            backup::commands::stop_backup,
            backup::commands::run_scan,
            backup::commands::run_backup,
            backup::commands::get_file_preview,
            backup::commands::get_home_dir,
            gphotos::gphotos_open,
            gphotos::gphotos_set_bounds,
            gphotos::gphotos_set_visible,
            gphotos::gphotos_get_zoom,
            gphotos::gphotos_zoom_step,
            gphotos::gphotos_zoom_reset,
            gphotos::gphotos_start,
            gphotos::gphotos_delete,
            gphotos::gphotos_pause,
            gphotos::gphotos_resume,
            gphotos::gphotos_stop,
            gphotos::gphotos_deselect,
            gphotos::gphotos_engine_event,
            gphotos::gphotos_page_status,
            gphotos::gphotos_diagnose,
            gphotos::gphotos_chrome_login,
            gphotos::gphotos_get_settings,
            gphotos::gphotos_save_settings,
            gphotos::gphotos_reset_settings,
            gphotos::gphotos_list_profiles,
            gphotos::gphotos_switch_profile,
            gphotos::gphotos_save_profile,
            gphotos::gphotos_delete_profile,
            gphotos::gphotos_get_dev_mode,
            gphotos::gphotos_set_dev_mode,
            gphotos::gphotos_get_tutorial_completed,
            gphotos::gphotos_set_tutorial_completed,
            gphotos::gphotos_log_history,
            gphotos::gphotos_log_clear,
        ])
        .setup(|app| {
            battery_catalog_sync::init_battery_catalog(app.handle());
            let handle = app.handle().clone();
            commands::start_device_poller(handle.clone());
            if let Some(window) = app.get_webview_window("main") {
                let emit_handle = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if adware::commands::close_allowed() {
                            return;
                        }
                        api.prevent_close();
                        let _ = emit_handle.emit("app-close-requested", ());
                    }
                });
            }
            adware::realtime::spawn(handle.clone());
            std::thread::spawn(move || {
                if let Some(state) = handle.try_state::<AdwareState>() {
                    adware::commands::reconcile_quiet(&state);
                }
                adware::commands::mark_startup_sync_done();
                let _ = handle.emit("sync-finished", ());
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
