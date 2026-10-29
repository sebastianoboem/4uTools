use tauri::{AppHandle, State};

use super::settings::SelectionParams;
use super::state::{emit_app, photos, push_log, EngineEvent, GphotosState};
use super::webview::wait_for_download;

pub async fn eval_value(webview: &tauri::Webview, js: &str) -> Result<serde_json::Value, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
    webview
        .eval_with_callback(js.to_string(), move |raw| {
            if let Some(tx) = slot.lock().expect("eval").take() {
                let _ = tx.send(raw);
            }
        })
        .map_err(|e| e.to_string())?;
    let raw = rx.await.map_err(|_| "valutazione interrotta".to_string())?;
    if raw.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&raw).map_err(|err| format!("{err}: {raw}"))
}

fn eval_call(app: &AppHandle, call: &str) -> Result<(), String> {
    photos(app)?.eval(call).map_err(|e| e.to_string())
}

pub fn start(app: &AppHandle, params: &SelectionParams, delete: bool) -> Result<(), String> {
    let payload = serde_json::to_string(params).map_err(|e| e.to_string())?;
    let method = if delete { "startAndDelete" } else { "start" };
    eval_call(app, &format!("window.__gfm?.{method}({payload})"))
}

pub fn control(app: &AppHandle, method: &str) -> Result<(), String> {
    eval_call(app, &format!("void window.__gfm?.{method}()"))
}

pub async fn page_status(app: &AppHandle) -> Result<serde_json::Value, String> {
    let Ok(webview) = photos(app) else {
        return Ok(serde_json::json!({
            "isPhotosGoogle": false,
            "isLoginBlocked": false
        }));
    };
    let value = eval_value(&webview, "window.__gfm?.pageStatus?.() ?? null").await?;
    if value.is_null() {
        return Ok(serde_json::json!({
            "isPhotosGoogle": false,
            "isLoginBlocked": false
        }));
    }
    Ok(value)
}

pub async fn diagnose(app: &AppHandle) -> Result<serde_json::Value, String> {
    let webview = photos(app)?;
    eval_value(&webview, "window.__gfm?.diagnose?.() ?? null").await
}

pub async fn on_engine_event(
    app: &AppHandle,
    state: &State<'_, GphotosState>,
    event: EngineEvent,
) -> Result<serde_json::Value, String> {
    match event {
        EngineEvent::Progress { payload } => {
            emit_app(app, "gphotos-progress", payload);
        }
        EngineEvent::Log { payload } => {
            let mut logs = state.logs.lock().expect("log");
            logs.push(payload.clone());
            if logs.len() > 400 {
                let drop_n = logs.len() - 400;
                logs.drain(0..drop_n);
            }
            drop(logs);
            emit_app(app, "gphotos-log", payload);
        }
        EngineEvent::Done { payload } => emit_app(app, "gphotos-done", payload),
        EngineEvent::Error { payload } => emit_app(app, "gphotos-error", payload),
        EngineEvent::PageStatus { payload } => emit_app(app, "gphotos-page-status", payload),
        EngineEvent::DownloadWait => {
            push_log(app, "info", "Attendo il download (max 15 minuti)");
            return wait_for_download(app).await;
        }
    }
    Ok(serde_json::Value::Null)
}
