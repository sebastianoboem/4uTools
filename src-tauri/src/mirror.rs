use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adb_bridge::AdbBridge;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Min time between frame captures (~15 fps cap). Actual rate depends on screencap latency.
const MIN_FRAME_INTERVAL: Duration = Duration::from_millis(66);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorFrame {
    pub serial: String,
    pub width: u32,
    pub height: u32,
    pub image_data_url: String,
}

struct ActiveMirror {
    cancel: Arc<AtomicBool>,
}

static MIRROR: Mutex<Option<ActiveMirror>> = Mutex::new(None);

pub fn stop_mirror_preview() {
    let mut guard = MIRROR.lock().expect("mirror lock");
    if let Some(session) = guard.take() {
        session.cancel.store(true, Ordering::SeqCst);
    }
}

pub fn start_mirror_preview(app: AppHandle, serial: String) -> Result<(), String> {
    stop_mirror_preview();

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut guard = MIRROR.lock().expect("mirror lock");
        *guard = Some(ActiveMirror {
            cancel: Arc::clone(&cancel),
        });
    }

    tauri::async_runtime::spawn(async move {
        loop {
            if cancel.load(Ordering::SeqCst) {
                break;
            }

            let frame_start = Instant::now();
            let serial_capture = serial.clone();
            let cancel_capture = Arc::clone(&cancel);

            let frame = tokio::task::spawn_blocking(move || capture_frame(&serial_capture, &cancel_capture))
                .await
                .ok()
                .flatten();

            if let Some(frame) = frame {
                let _ = app.emit("mirror-frame", frame);
            }

            let elapsed = frame_start.elapsed();
            if elapsed < MIN_FRAME_INTERVAL {
                tokio::time::sleep(MIN_FRAME_INTERVAL - elapsed).await;
            }
        }
    });

    Ok(())
}

fn capture_frame(serial: &str, cancel: &AtomicBool) -> Option<MirrorFrame> {
    if cancel.load(Ordering::SeqCst) {
        return None;
    }

    let bridge = AdbBridge::with_serial(serial).ok()?;
    let png = bridge.exec_out("screencap -p").ok()?;
    if png.len() <= 24 {
        return None;
    }

    let (width, height) = png_dimensions(&png)?;
    let encoded = B64.encode(&png);

    Some(MirrorFrame {
        serial: serial.to_string(),
        width,
        height,
        image_data_url: format!("data:image/png;base64,{encoded}"),
    })
}

pub fn mirror_tap(serial: &str, x: i32, y: i32) -> Result<(), String> {
    let bridge = AdbBridge::with_serial(serial).map_err(|e| e.to_string())?;
    bridge
        .shell(&format!("input tap {x} {y}"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || data.get(0..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let w = u32::from_be_bytes(data[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(data[20..24].try_into().ok()?);
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_png_header() {
        let mut png = vec![0u8; 24];
        png[0..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&1080u32.to_be_bytes());
        png[20..24].copy_from_slice(&2400u32.to_be_bytes());
        assert_eq!(png_dimensions(&png), Some((1080, 2400)));
    }
}
