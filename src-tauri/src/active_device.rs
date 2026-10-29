use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct ActiveDevice(pub Mutex<Option<String>>);

#[derive(Clone, Serialize)]
struct ActiveDeviceEvent {
    serial: Option<String>,
}

#[tauri::command]
pub fn set_active_device(
    app: AppHandle,
    state: State<'_, ActiveDevice>,
    serial: Option<String>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = serial.clone();
    drop(guard);
    app.emit(
        "active-device-changed",
        ActiveDeviceEvent { serial },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_device(state: State<'_, ActiveDevice>) -> Result<Option<String>, String> {
    state.0.lock().map(|g| g.clone()).map_err(|e| e.to_string())
}
