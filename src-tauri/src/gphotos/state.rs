use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

pub const WEBVIEW_LABEL: &str = "gphotos";
pub const ZOOM_MIN: f64 = 0.3;
pub const ZOOM_MAX: f64 = 3.0;
pub const ZOOM_STEP: f64 = 0.1;
pub const ZOOM_DEFAULT: f64 = 0.5;

pub struct GphotosState {
    pub zoom: Mutex<f64>,
    pub download_tx: Mutex<Option<tokio::sync::oneshot::Sender<bool>>>,
    pub download_done: Mutex<Option<(Instant, bool)>>,
    pub logs: Mutex<Vec<serde_json::Value>>,
    pub root: PathBuf,
}

impl GphotosState {
    pub fn new() -> Self {
        let root = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("4uTools")
            .join("gphotos");
        let _ = std::fs::create_dir_all(&root);
        Self {
            zoom: Mutex::new(ZOOM_DEFAULT),
            download_tx: Mutex::new(None),
            download_done: Mutex::new(None),
            logs: Mutex::new(Vec::new()),
            root,
        }
    }
}

impl GphotosState {
    pub fn zoom_factor(&self) -> f64 {
        *self.zoom.lock().expect("zoom")
    }

    pub fn set_zoom_factor(&self, factor: f64) {
        *self.zoom.lock().expect("zoom") = factor;
    }
}

pub fn emit_app(app: &AppHandle, event: &str, payload: impl Serialize) {
    if let Ok(value) = serde_json::to_value(&payload) {
        let _ = app.emit(event, value);
    }
}

pub fn photos(app: &AppHandle) -> Result<tauri::Webview, String> {
    app.get_webview(WEBVIEW_LABEL)
        .ok_or_else(|| "Vista Google Foto non aperta.".to_string())
}

pub fn push_log(app: &AppHandle, level: &str, msg: &str) {
    let entry = serde_json::json!({
        "ts": chrono_millis(),
        "level": level,
        "msg": msg,
    });
    if let Some(state) = app.try_state::<GphotosState>() {
        let mut logs = state.logs.lock().expect("log");
        logs.push(entry.clone());
        if logs.len() > 400 {
            let drop_n = logs.len() - 400;
            logs.drain(0..drop_n);
        }
    }
    emit_app(app, "gphotos-log", entry);
}

fn chrono_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoomState {
    pub factor: f64,
    pub percent: i64,
}

pub fn clamp_zoom(factor: f64) -> f64 {
    factor.clamp(ZOOM_MIN, ZOOM_MAX)
}

pub fn zoom_state(factor: f64) -> ZoomState {
    let factor = clamp_zoom(factor);
    ZoomState {
        percent: (factor * 100.0).round() as i64,
        factor,
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EngineEvent {
    Progress { payload: serde_json::Value },
    Log { payload: serde_json::Value },
    Done { payload: serde_json::Value },
    Error { payload: serde_json::Value },
    PageStatus { payload: serde_json::Value },
    DownloadWait,
}
