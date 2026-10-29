mod chrome_login;
mod engine;
mod profiles;
mod settings;
mod state;
mod webview;

use tauri::{AppHandle, State};

use settings::{load_app_settings, load_settings, save_app_settings, save_settings, AppSettings, SelectionParams};
use state::{EngineEvent, ZoomState};

pub use state::GphotosState;

#[tauri::command]
pub fn gphotos_open(app: AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    webview::open_photos(&app, x, y, width, height)
}

#[tauri::command]
pub fn gphotos_set_bounds(app: AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    webview::set_bounds(&app, x, y, width, height)
}

#[tauri::command]
pub fn gphotos_set_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    webview::show_photos(&app, visible)
}

#[tauri::command]
pub fn gphotos_get_zoom(app: AppHandle) -> ZoomState {
    webview::current_zoom(&app)
}

#[tauri::command]
pub fn gphotos_zoom_step(app: AppHandle, direction: i8) -> Result<ZoomState, String> {
    webview::step_zoom(&app, direction)
}

#[tauri::command]
pub fn gphotos_zoom_reset(app: AppHandle) -> Result<ZoomState, String> {
    webview::reset_zoom(&app)
}

#[tauri::command]
pub fn gphotos_start(app: AppHandle, params: SelectionParams) -> Result<(), String> {
    engine::start(&app, &params, false)
}

#[tauri::command]
pub fn gphotos_delete(app: AppHandle, params: SelectionParams) -> Result<(), String> {
    engine::start(&app, &params, true)
}

#[tauri::command]
pub fn gphotos_pause(app: AppHandle) -> Result<(), String> {
    engine::control(&app, "pause")
}

#[tauri::command]
pub fn gphotos_resume(app: AppHandle) -> Result<(), String> {
    engine::control(&app, "resume")
}

#[tauri::command]
pub fn gphotos_stop(app: AppHandle) -> Result<(), String> {
    engine::control(&app, "stop")
}

#[tauri::command]
pub fn gphotos_deselect(app: AppHandle) -> Result<(), String> {
    engine::control(&app, "deselect")
}

#[tauri::command]
pub async fn gphotos_engine_event(
    app: AppHandle,
    state: State<'_, GphotosState>,
    event: EngineEvent,
) -> Result<serde_json::Value, String> {
    engine::on_engine_event(&app, &state, event).await
}

#[tauri::command]
pub async fn gphotos_page_status(app: AppHandle) -> Result<serde_json::Value, String> {
    engine::page_status(&app).await
}

#[tauri::command]
pub async fn gphotos_diagnose(app: AppHandle) -> Result<serde_json::Value, String> {
    engine::diagnose(&app).await
}

#[tauri::command]
pub async fn gphotos_chrome_login(app: AppHandle) -> Result<serde_json::Value, String> {
    match chrome_login::run_chrome_login(&app).await {
        Ok(cookie_count) => Ok(serde_json::json!({ "ok": true, "cookieCount": cookie_count })),
        Err(message) => Ok(serde_json::json!({ "ok": false, "message": message })),
    }
}

#[tauri::command]
pub fn gphotos_get_settings(state: State<'_, GphotosState>) -> SelectionParams {
    load_settings(&state)
}

#[tauri::command]
pub fn gphotos_save_settings(state: State<'_, GphotosState>, params: SelectionParams) -> Result<SelectionParams, String> {
    save_settings(&state, params)
}

#[tauri::command]
pub fn gphotos_reset_settings(state: State<'_, GphotosState>) -> Result<SelectionParams, String> {
    save_settings(&state, SelectionParams::default())
}

#[tauri::command]
pub fn gphotos_list_profiles(state: State<'_, GphotosState>) -> profiles::ProfileStoreData {
    profiles::load_store(&state)
}

#[tauri::command]
pub async fn gphotos_switch_profile(
    app: AppHandle,
    profile_id: Option<String>,
) -> Result<profiles::ProfileStoreData, String> {
    profiles::switch_profile(&app, profile_id).await
}

#[tauri::command]
pub async fn gphotos_save_profile(app: AppHandle, name: String) -> Result<serde_json::Value, String> {
    match profiles::save_named_profile(&app, &name).await {
        Ok(store) => Ok(serde_json::json!({ "ok": true, "store": store })),
        Err(message) => Ok(serde_json::json!({ "ok": false, "message": message })),
    }
}

#[tauri::command]
pub fn gphotos_delete_profile(app: AppHandle, profile_id: String) -> Result<serde_json::Value, String> {
    match profiles::delete_named_profile(&app, &profile_id) {
        Ok(store) => Ok(serde_json::json!({ "ok": true, "store": store })),
        Err(message) => Ok(serde_json::json!({ "ok": false, "message": message })),
    }
}

#[tauri::command]
pub fn gphotos_get_dev_mode(state: State<'_, GphotosState>) -> serde_json::Value {
    serde_json::json!({ "enabled": load_app_settings(&state).dev_mode })
}

#[tauri::command]
pub fn gphotos_set_dev_mode(app: AppHandle, state: State<'_, GphotosState>, enabled: bool) -> Result<serde_json::Value, String> {
    let mut settings = load_app_settings(&state);
    settings.dev_mode = enabled;
    save_app_settings(&state, &settings)?;
    state::emit_app(&app, "gphotos-dev-mode", serde_json::json!({ "enabled": enabled }));
    Ok(serde_json::json!({ "enabled": enabled }))
}

#[tauri::command]
pub fn gphotos_get_tutorial_completed(state: State<'_, GphotosState>) -> serde_json::Value {
    serde_json::json!({ "completed": load_app_settings(&state).tutorial_completed })
}

#[tauri::command]
pub fn gphotos_set_tutorial_completed(
    state: State<'_, GphotosState>,
    completed: bool,
) -> Result<serde_json::Value, String> {
    let mut settings: AppSettings = load_app_settings(&state);
    settings.tutorial_completed = completed;
    save_app_settings(&state, &settings)?;
    Ok(serde_json::json!({ "completed": completed }))
}

#[tauri::command]
pub fn gphotos_log_history(state: State<'_, GphotosState>) -> Vec<serde_json::Value> {
    state.logs.lock().expect("log").clone()
}

#[tauri::command]
pub fn gphotos_log_clear(state: State<'_, GphotosState>) -> Result<(), String> {
    state.logs.lock().expect("log").clear();
    Ok(())
}
