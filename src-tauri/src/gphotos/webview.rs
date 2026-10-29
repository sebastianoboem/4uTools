use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Rect, Url, WebviewUrl};

use super::state::{
    clamp_zoom, emit_app, photos, zoom_state, GphotosState, ZoomState, WEBVIEW_LABEL, ZOOM_DEFAULT,
    ZOOM_STEP,
};

const ENGINE_JS: &str = include_str!("../../gen-assets/gfm-engine.js");
const PHOTOS_URL: &str = "https://photos.google.com/";

fn allow_navigation(url: &Url) -> bool {
    if !matches!(url.scheme(), "https" | "http") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    host == "google.com"
        || host.ends_with(".google.com")
        || host.ends_with(".googleusercontent.com")
        || host == "gstatic.com"
        || host.ends_with(".gstatic.com")
}

pub fn reload_photos(app: &AppHandle) -> Result<(), String> {
    let webview = photos(app)?;
    let url = PHOTOS_URL.parse::<Url>().map_err(|e| e.to_string())?;
    webview.navigate(url).map_err(|e| e.to_string())
}

fn apply_zoom(app: &AppHandle, factor: f64) -> Result<ZoomState, String> {
    let factor = clamp_zoom(factor);
    app.state::<GphotosState>().set_zoom_factor(factor);
    if let Ok(webview) = photos(app) {
        webview.set_zoom(factor).map_err(|e| e.to_string())?;
    }
    let zoom = zoom_state(factor);
    emit_app(app, "gphotos-zoom", &zoom);
    Ok(zoom)
}

pub fn current_zoom(app: &AppHandle) -> ZoomState {
    zoom_state(app.state::<GphotosState>().zoom_factor())
}

pub fn step_zoom(app: &AppHandle, direction: i8) -> Result<ZoomState, String> {
    let current = app.state::<GphotosState>().zoom_factor();
    let delta = if direction < 0 { -ZOOM_STEP } else { ZOOM_STEP };
    apply_zoom(app, current + delta)
}

pub fn reset_zoom(app: &AppHandle) -> Result<ZoomState, String> {
    apply_zoom(app, ZOOM_DEFAULT)
}

fn note_download(app: &AppHandle, success: bool) {
    let state = app.state::<GphotosState>();
    let pending = state.download_tx.lock().expect("download").take();
    if let Some(tx) = pending {
        let _ = tx.send(success);
    } else {
        *state.download_done.lock().expect("download") = Some((Instant::now(), success));
    }
}

pub async fn wait_for_download(app: &AppHandle) -> Result<serde_json::Value, String> {
    let recent = app.state::<GphotosState>().download_done.lock().expect("download").take();
    if let Some((when, success)) = recent {
        if when.elapsed() < Duration::from_secs(30) {
            return Ok(download_payload(success, None));
        }
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.state::<GphotosState>().download_tx.lock().expect("download") = Some(tx);
    match tokio::time::timeout(Duration::from_secs(15 * 60), rx).await {
        Ok(Ok(true)) => Ok(download_payload(true, None)),
        Ok(Ok(false)) => Ok(download_payload(false, Some("failed"))),
        _ => {
            let _ = app.state::<GphotosState>().download_tx.lock().expect("download").take();
            Ok(download_payload(false, Some("timeout")))
        }
    }
}

fn download_payload(ok: bool, step: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "ok": ok,
        "method": if ok { Some("menu") } else { None::<&str> },
        "step": step,
    })
}

pub fn open_photos(app: &AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    if app.get_webview(WEBVIEW_LABEL).is_some() {
        set_bounds(app, x, y, width, height)?;
        let _ = show_photos(app, true);
        return Ok(());
    }
    let window = app
        .get_window("main")
        .ok_or_else(|| "Finestra principale assente.".to_string())?;
    let handle = app.clone();
    let download_handle = app.clone();
    let url = PHOTOS_URL.parse::<Url>().map_err(|e| e.to_string())?;
    let mut builder = tauri::webview::WebviewBuilder::new(WEBVIEW_LABEL, WebviewUrl::External(url))
        .initialization_script(ENGINE_JS)
        .on_navigation(allow_navigation)
        .on_new_window(move |url, _features| {
            if allow_navigation(&url) {
                if let Some(webview) = handle.get_webview(WEBVIEW_LABEL) {
                    let _ = webview.navigate(url);
                }
            }
            tauri::webview::NewWindowResponse::Deny
        })
        .on_download(move |_webview, event| match event {
            tauri::webview::DownloadEvent::Requested { destination, .. } => {
                if let Some(dir) = dirs::download_dir() {
                    let name = destination
                        .file_name()
                        .map(|n| n.to_os_string())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "google-photos.zip".into());
                    *destination = dir.join(name);
                }
                true
            }
            tauri::webview::DownloadEvent::Finished { success, .. } => {
                note_download(&download_handle, success);
                false
            }
            _ => true,
        });

    #[cfg(target_os = "macos")]
    {
        builder = builder.data_store_identifier(*b"4uTools-gphotos!");
    }
    #[cfg(not(target_os = "macos"))]
    {
        let dir = app.state::<GphotosState>().root.join("webview");
        let _ = std::fs::create_dir_all(&dir);
        builder = builder.data_directory(dir);
    }

    window
        .add_child(
            builder,
            LogicalPosition::new(x, y),
            LogicalSize::new(width.max(1.0), height.max(1.0)),
        )
        .map_err(|e| e.to_string())?;
    let _ = apply_zoom(app, current_zoom(app).factor);
    Ok(())
}

pub fn set_bounds(app: &AppHandle, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let Ok(webview) = photos(app) else {
        return Ok(());
    };
    if width < 2.0 || height < 2.0 {
        return Ok(());
    }
    webview
        .set_bounds(Rect {
            position: LogicalPosition::new(x, y).into(),
            size: LogicalSize::new(width, height).into(),
        })
        .map_err(|e| e.to_string())
}

pub fn show_photos(app: &AppHandle, visible: bool) -> Result<(), String> {
    let Ok(webview) = photos(app) else {
        return Ok(());
    };
    if visible {
        webview.show().map_err(|e| e.to_string())
    } else {
        webview.hide().map_err(|e| e.to_string())
    }
}
