mod commands;
mod mirror;
mod partner_app;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_adb_status,
            commands::list_devices,
            commands::load_device_summary,
            commands::reboot_device,
            commands::shutdown_device,
            commands::start_mirror_preview,
            commands::stop_mirror_preview,
            commands::mirror_tap,
            commands::autobackup::check_autobackup,
            commands::autobackup::install_autobackup,
            commands::autobackup::launch_autobackup,
            commands::app_manager::check_app_manager,
            commands::app_manager::install_app_manager,
            commands::app_manager::launch_app_manager,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            commands::start_device_poller(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
