use crate::mirror;
use adb_bridge::{AdbBridge, AdbDevice, DeviceState};
use device_info::DeviceSummary;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub mod app_manager;
pub mod autobackup;

#[derive(Debug, Serialize)]
pub struct AdbStatus {
    pub adb_path: Option<String>,
    pub devices: Vec<AdbDevice>,
    pub has_authorized_device: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_adb_status() -> Result<AdbStatus, String> {
    let bridge = AdbBridge::new().map_err(|e| e.to_string())?;
    let devices = bridge.list_devices().map_err(|e| e.to_string())?;
    let has_authorized_device = devices.iter().any(|d| d.state == DeviceState::Device);
    Ok(AdbStatus {
        adb_path: Some(bridge.adb_path().to_string_lossy().into_owned()),
        devices,
        has_authorized_device,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_devices() -> Result<Vec<AdbDevice>, String> {
    let bridge = AdbBridge::new().map_err(|e| e.to_string())?;
    bridge.list_devices().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub fn load_device_summary(serial: String) -> Result<DeviceSummary, String> {
    device_info::load_device_summary(&serial)
}

#[tauri::command(rename_all = "snake_case")]
pub fn reboot_device(serial: String) -> Result<(), String> {
    let bridge = AdbBridge::with_serial(serial).map_err(|e| e.to_string())?;
    bridge.reboot().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub fn shutdown_device(serial: String) -> Result<(), String> {
    let bridge = AdbBridge::with_serial(serial).map_err(|e| e.to_string())?;
    bridge.shutdown().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub fn start_mirror_preview(app: AppHandle, serial: String) -> Result<(), String> {
    mirror::start_mirror_preview(app, serial)
}

#[tauri::command(rename_all = "snake_case")]
pub fn stop_mirror_preview() -> Result<(), String> {
    mirror::stop_mirror_preview();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn mirror_tap(
    serial: String,
    x: f64,
    y: f64,
    display_width: f64,
    display_height: f64,
    device_width: u32,
    device_height: u32,
) -> Result<(), String> {
    if display_width <= 0.0 || display_height <= 0.0 || device_width == 0 || device_height == 0 {
        return Err("Dimensioni schermo non valide".into());
    }
    let tx = ((x / display_width) * device_width as f64).round() as i32;
    let ty = ((y / display_height) * device_height as f64).round() as i32;
    mirror::mirror_tap(&serial, tx, ty)
}

pub fn start_device_poller(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(devices) = list_devices() {
                let _ = app.emit("devices-changed", devices);
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
