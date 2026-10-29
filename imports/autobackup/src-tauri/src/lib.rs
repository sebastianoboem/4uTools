mod commands;

use commands::AppState;
use autobackup_core::BackupController;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            controller: BackupController::new(),
            scanned_files: Mutex::new(vec![]),
            preview_cache: Mutex::new(HashMap::new()),
            preview_bytes_cache: Mutex::new(HashMap::new()),
            preview_prefix_cache: Mutex::new(HashMap::new()),
            android_preview_semaphore: Arc::new(Semaphore::new(4)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_categories,
            commands::get_default_deselected_extensions,
            commands::get_default_exclusions,
            commands::get_default_exceptions,
            commands::list_removable_drives,
            commands::get_drive_for_path,
            commands::check_adb,
            commands::list_android_devices,
            commands::pause_backup,
            commands::resume_backup,
            commands::stop_backup,
            commands::run_scan,
            commands::run_backup,
            commands::get_file_preview,
            commands::get_home_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
