use autobackup_core::{
    apply_test_mode_limit, categories_in_display_order, copy_android_files, copy_files, default_android_exceptions,
    default_deselected_extensions, drive_info_for_path, find_adb,
    get_devices, get_removable_drives, is_android_device_path, read_android_file,
    read_android_file_prefix, resolve_scan_roots, scan_android_files, scan_files,
    system_folders, android_system_folders, BackupController, DriveInfo, FileEntry,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct AppState {
    pub controller: BackupController,
    pub scanned_files: Mutex<Vec<FileEntry>>,
    pub preview_cache: Mutex<HashMap<String, String>>,
    pub preview_bytes_cache: Mutex<HashMap<String, Vec<u8>>>,
    pub preview_prefix_cache: Mutex<HashMap<String, Vec<u8>>>,
    pub android_preview_semaphore: Arc<Semaphore>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanRequest {
    pub mode: String,
    pub category_map: HashMap<String, Vec<String>>,
    pub custom_extensions: Vec<String>,
    pub exclusions: Vec<String>,
    pub exceptions: Vec<String>,
    pub whitelist_paths: Vec<String>,
    pub destination_mount: String,
    pub device_id: Option<String>,
    pub test_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub files: Vec<FileEntry>,
    pub total_size: u64,
    pub scan_roots: Vec<String>,
}

#[tauri::command(rename = "backup_get_categories")]
pub fn get_categories() -> Vec<CategoryInfo> {
    categories_in_display_order()
        .into_iter()
        .map(|(name, exts)| CategoryInfo {
            name: name.to_string(),
            extensions: exts.into_iter().map(String::from).collect(),
        })
        .collect()
}

#[tauri::command(rename = "backup_get_default_deselected_extensions")]
pub fn get_default_deselected_extensions() -> Vec<String> {
    default_deselected_extensions()
        .into_iter()
        .map(String::from)
        .collect()
}

#[tauri::command(rename = "backup_get_default_exclusions")]
pub fn get_default_exclusions(mode: String) -> Vec<String> {
    if mode == "android" {
        android_system_folders()
    } else {
        system_folders()
    }
}

#[tauri::command(rename = "backup_get_default_exceptions")]
pub fn get_default_exceptions(mode: String) -> Vec<String> {
    if mode == "android" {
        default_android_exceptions()
    } else {
        vec![]
    }
}

#[tauri::command(rename = "backup_list_removable_drives")]
pub fn list_removable_drives() -> Vec<DriveInfo> {
    get_removable_drives()
}

#[tauri::command(rename = "backup_get_drive_for_path")]
pub fn get_drive_for_path(path: String) -> Option<DriveInfo> {
    drive_info_for_path(&path)
}

#[tauri::command(rename = "backup_check_adb")]
pub fn check_adb() -> Option<String> {
    find_adb()
}

#[tauri::command(rename = "backup_list_android_devices")]
pub async fn list_android_devices() -> Vec<autobackup_core::AndroidDevice> {
    if find_adb().is_none() {
        return vec![];
    }
    get_devices().await
}

#[tauri::command(rename = "backup_pause_backup")]
pub fn pause_backup(state: State<'_, AppState>) {
    state.controller.pause();
}

#[tauri::command(rename = "backup_resume_backup")]
pub fn resume_backup(state: State<'_, AppState>) {
    state.controller.resume();
}

#[tauri::command(rename = "backup_stop_backup")]
pub fn stop_backup(state: State<'_, AppState>) {
    state.controller.stop();
}

#[tauri::command(rename = "backup_run_scan")]
pub async fn run_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ScanRequest,
) -> Result<ScanResult, String> {
    state.controller.reset();

    let scan_roots = if request.mode == "android" {
        vec!["Android Device".into()]
    } else {
        resolve_scan_roots(&request.whitelist_paths, &request.destination_mount)
    };

    let progress_app = app.clone();
    let progress_cb: autobackup_core::scanner::ScanCallback = Box::new(move |p| {
        let _ = progress_app.emit("scan-progress", p);
    });

    let (mut files, mut total_size) = if request.mode == "android" {
        let device_id = request
            .device_id
            .clone()
            .ok_or_else(|| "Nessun dispositivo Android selezionato".to_string())?;
        if find_adb().is_none() {
            return Err("ADB non trovato".into());
        }
        scan_android_files(
            &device_id,
            &request.category_map,
            &request.custom_extensions,
            &request.exclusions,
            &request.exceptions,
            &state.controller,
            Some(&progress_cb),
        )
        .await
    } else {
        let roots: Vec<String> = if scan_roots == vec!["Android Device".to_string()] {
            request.whitelist_paths.clone()
        } else {
            scan_roots
                .iter()
                .filter(|r| *r != "Android Device")
                .cloned()
                .collect()
        };

        if roots.is_empty() {
            return Err("Nessun percorso di scansione definito".into());
        }

        scan_files(
            &roots,
            &request.category_map,
            &request.custom_extensions,
            &request.exclusions,
            &request.exceptions,
            &state.controller,
            Some(&progress_cb),
        )
    };

    if request.test_mode {
        (files, total_size) = apply_test_mode_limit(files, 10);
    }

    *state.scanned_files.lock().map_err(|e| e.to_string())? = files.clone();
    state.preview_cache.lock().map_err(|e| e.to_string())?.clear();
    state
        .preview_bytes_cache
        .lock()
        .map_err(|e| e.to_string())?
        .clear();
    state
        .preview_prefix_cache
        .lock()
        .map_err(|e| e.to_string())?
        .clear();

    if request.mode == "android" {
        let warm_paths: Vec<String> = files
            .iter()
            .filter(|f| f.category == "Immagini")
            .take(32)
            .map(|f| f.source.clone())
            .collect();
        let warm_app = app.clone();
        tokio::spawn(async move {
            for path in warm_paths {
                let state = warm_app.state::<AppState>();
                let _ = resolve_file_preview(&state, path, Some(280)).await;
            }
        });
    }

    if request.mode != "android" {
        let scope = app.asset_protocol_scope();
        for root in &scan_roots {
            if root != "Android Device" {
                let _ = scope.allow_directory(root, true);
            }
        }
        for file in &files {
            let _ = scope.allow_file(&file.source);
        }
    }

    Ok(ScanResult {
        files,
        total_size,
        scan_roots: if request.mode == "android" {
            vec!["/sdcard".into()]
        } else {
            scan_roots
        },
    })
}

#[tauri::command(rename = "backup_run_backup")]
pub async fn run_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
    destination: String,
    destination_is_drive_root: bool,
    verify: bool,
    files: Vec<FileEntry>,
) -> Result<autobackup_core::BackupResult, String> {
    state.controller.reset();

    if files.is_empty() {
        return Err("Nessun file selezionato per il backup.".into());
    }

    let progress_app = app.clone();
    let progress_cb: autobackup_core::copier::CopyCallback = Box::new(move |p| {
        let _ = progress_app.emit("backup-progress", p);
    });

    let (copied_count, errors, dest_folder) = if mode == "android" {
        if find_adb().is_none() {
            return Err("ADB non trovato".into());
        }
        copy_android_files(
            &files,
            &destination,
            destination_is_drive_root,
            &state.controller,
            Some(&progress_cb),
        )
        .await
    } else {
        copy_files(
            &files,
            &destination,
            destination_is_drive_root,
            verify,
            &state.controller,
            Some(&progress_cb),
        )
    };

    if !errors.is_empty() {
        let log_path = std::env::current_dir()
            .unwrap_or_default()
            .join("backup_errors.log");
        let content = errors.join("\n");
        let _ = std::fs::write(&log_path, content);
    }

    let result = autobackup_core::BackupResult {
        copied_count,
        destination: dest_folder,
        errors,
    };

    let _ = app.emit("backup-complete", &result);
    Ok(result)
}

#[tauri::command(rename = "backup_get_file_preview")]
pub async fn get_file_preview(
    path: String,
    max_side: Option<u32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    resolve_file_preview(&state, path, max_side).await
}

async fn resolve_file_preview(
    state: &AppState,
    path: String,
    max_side: Option<u32>,
) -> Result<String, String> {
    let cache_key = format!("{}:{}", path, max_side.unwrap_or(0));
    if let Some(cached) = state
        .preview_cache
        .lock()
        .map_err(|e| e.to_string())?
        .get(&cache_key)
        .cloned()
    {
        return Ok(cached);
    }

    let entry = {
        let files = state.scanned_files.lock().map_err(|e| e.to_string())?;
        files
            .iter()
            .find(|f| f.source == path)
            .cloned()
            .ok_or_else(|| "Anteprima non autorizzata per questo file.".to_string())?
    };

    const MAX_PREVIEW_BYTES: u64 = 30 * 1024 * 1024;
    const EXIF_PREFIX_BYTES: u64 = 768 * 1024;
    const LIGHT_PARTIAL_BYTES: u64 = 2 * 1024 * 1024;
    let max_side_val = max_side.unwrap_or(280);
    let thumb_request = max_side_val <= 400;
    let light_request = max_side_val > 400 && max_side_val <= 1280;

    let cached_bytes = {
        state
            .preview_bytes_cache
            .lock()
            .map_err(|e| e.to_string())?
            .get(&path)
            .cloned()
    };
    let (bytes, skip_resize) = if let Some(cached) = cached_bytes {
        (cached, false)
    } else {
        let (loaded, skip_resize) = if is_android_device_path(&path) {
            let device_id = entry
                .device_id
                .as_ref()
                .ok_or_else(|| "ID dispositivo Android mancante.".to_string())?;
            if find_adb().is_none() {
                return Err("ADB non trovato.".into());
            }
            let semaphore = state.android_preview_semaphore.clone();
            if thumb_request && is_jpeg_path(&path) {
                let _permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| e.to_string())?;
                let prefix =
                    read_android_file_prefix(device_id, &path, EXIF_PREFIX_BYTES).await?;
                state
                    .preview_prefix_cache
                    .lock()
                    .map_err(|e| e.to_string())?
                    .insert(path.clone(), prefix.clone());
                if let Some(thumb) = extract_exif_thumbnail(&prefix) {
                    (thumb, false)
                } else {
                    drop(_permit);
                    let _permit = semaphore
                        .acquire_owned()
                        .await
                        .map_err(|e| e.to_string())?;
                    let full =
                        read_android_file(device_id, &path, MAX_PREVIEW_BYTES).await?;
                    state
                        .preview_bytes_cache
                        .lock()
                        .map_err(|e| e.to_string())?
                        .insert(path.clone(), full.clone());
                    (full, false)
                }
            } else if light_request && is_jpeg_path(&path) {
                load_android_lightbox_bytes(
                    state,
                    &semaphore,
                    device_id,
                    &path,
                    max_side_val,
                    LIGHT_PARTIAL_BYTES,
                )
                .await?
            } else {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|e| e.to_string())?;
                let full = read_android_file(device_id, &path, MAX_PREVIEW_BYTES).await?;
                state
                    .preview_bytes_cache
                    .lock()
                    .map_err(|e| e.to_string())?
                    .insert(path.clone(), full.clone());
                (full, false)
            }
        } else {
            let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
            if meta.len() > MAX_PREVIEW_BYTES {
                return Err("File troppo grande per l'anteprima.".into());
            }
            (std::fs::read(&path).map_err(|e| e.to_string())?, false)
        };
        (loaded, skip_resize)
    };

    let path_for_fallback = path.clone();
    let bytes_for_resize = bytes.clone();
    let (out_bytes, mime) = if skip_resize {
        (bytes, "image/jpeg".to_string())
    } else {
        tokio::task::spawn_blocking(
            move || -> Result<(Vec<u8>, String), String> {
                match resize_image_preview(&bytes_for_resize, max_side_val) {
                    Ok((out, mime)) => Ok((out, mime.to_string())),
                    Err(_) => {
                        let mime = preview_mime_for_path(&path_for_fallback).to_string();
                        Ok((bytes_for_resize, mime))
                    }
                }
            },
        )
        .await
        .map_err(|e| e.to_string())??
    };

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let data_url = format!("data:{mime};base64,{}", STANDARD.encode(out_bytes));
    state
        .preview_cache
        .lock()
        .map_err(|e| e.to_string())?
        .insert(cache_key, data_url.clone());
    Ok(data_url)
}

fn is_jpeg_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "jpe" | "jfif"
            )
        })
        .unwrap_or(false)
}

async fn get_android_prefix(
    state: &AppState,
    semaphore: &Arc<Semaphore>,
    device_id: &str,
    path: &str,
    min_bytes: u64,
) -> Result<Vec<u8>, String> {
    if let Some(cached) = state
        .preview_prefix_cache
        .lock()
        .map_err(|e| e.to_string())?
        .get(path)
        .cloned()
    {
        if cached.len() as u64 >= min_bytes.saturating_mul(9) / 10 {
            return Ok(cached);
        }
    }
    let _permit = semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| e.to_string())?;
    let prefix = read_android_file_prefix(device_id, path, min_bytes).await?;
    state
        .preview_prefix_cache
        .lock()
        .map_err(|e| e.to_string())?
        .insert(path.to_string(), prefix.clone());
    Ok(prefix)
}

async fn load_android_lightbox_bytes(
    state: &AppState,
    semaphore: &Arc<Semaphore>,
    device_id: &str,
    path: &str,
    max_side: u32,
    partial_bytes: u64,
) -> Result<(Vec<u8>, bool), String> {
    let prefix =
        get_android_prefix(state, semaphore, device_id, path, partial_bytes).await?;

    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        if let Some(jpeg) = try_jpeg_preview_from_prefix(&prefix, max_side) {
            return Ok(jpeg);
        }
        let exif_thumb = extract_exif_thumbnail(&prefix)
            .ok_or_else(|| "Anteprima ingrandita non disponibile per questo file.".to_string())?;
        let (resized, _) = resize_image_preview_lightbox(&exif_thumb, max_side)?;
        Ok(resized)
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok((bytes, true))
}

fn extract_exif_thumbnail(data: &[u8]) -> Option<Vec<u8>> {
    use exif::{In, Reader, Tag};

    let mut cursor = Cursor::new(data);
    let exif = Reader::new().read_from_container(&mut cursor).ok()?;
    let offset = exif
        .get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL)
        .and_then(|f| f.value.get_uint(0))? as usize;
    let length = exif
        .get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL)
        .and_then(|f| f.value.get_uint(0))? as usize;
    if length == 0 {
        return None;
    }
    let buf = exif.buf();
    if offset + length <= buf.len() {
        let thumb = &buf[offset..offset + length];
        if is_jpeg_bytes(thumb) {
            return Some(thumb.to_vec());
        }
    }
    find_secondary_jpeg(data)
}

fn find_secondary_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    let mut first = None;
    let mut pos = 0usize;
    while pos + 1 < data.len() {
        if data[pos] == 0xFF && data[pos + 1] == 0xD8 {
            if first.is_some() {
                let end = (pos + 2..data.len())
                    .find(|&i| i + 1 < data.len() && data[i] == 0xFF && data[i + 1] == 0xD9)?;
                return Some(data[pos..=end + 1].to_vec());
            }
            first = Some(pos);
        }
        pos += 1;
    }
    None
}

fn try_jpeg_preview_from_prefix(prefix: &[u8], max_side: u32) -> Option<Vec<u8>> {
    if !is_jpeg_bytes(prefix) {
        return None;
    }
    let (out, _) = resize_jpeg_preview(prefix, max_side).ok()?;
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(&out));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let longest = info.width.max(info.height) as u32;
    if longest < max_side.saturating_mul(3) / 5 {
        return None;
    }
    Some(out)
}

fn is_jpeg_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8
}

fn fit_rgb_to_max_side(rgb: image::RgbImage, max_side: u32) -> image::RgbImage {
    let w = rgb.width();
    let h = rgb.height();
    if w == 0 || h == 0 {
        return rgb;
    }
    let longest = w.max(h);
    if longest == max_side {
        return rgb;
    }
    if longest > max_side {
        return image::imageops::thumbnail(&rgb, max_side, max_side);
    }
    let scale = max_side as f64 / longest as f64;
    let new_w = ((w as f64 * scale).round() as u32).max(1);
    let new_h = ((h as f64 * scale).round() as u32).max(1);
    let resized =
        image::imageops::resize(&rgb, new_w, new_h, image::imageops::FilterType::Lanczos3);
    resized
}

fn jpeg_quality_for_max_side(max_side: u32) -> u8 {
    if max_side > 400 { 92 } else { 72 }
}

fn encode_rgb_jpeg(rgb: image::RgbImage, quality: u8) -> Result<(Vec<u8>, &'static str), String> {
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    encoder
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    Ok((out, "image/jpeg"))
}

fn resize_image_preview_lightbox(bytes: &[u8], max_side: u32) -> Result<(Vec<u8>, &'static str), String> {
    if is_jpeg_bytes(bytes) {
        if let Ok(resized) = resize_jpeg_preview(bytes, max_side) {
            return Ok(resized);
        }
    }

    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    let fitted = fit_rgb_to_max_side(img.to_rgb8(), max_side);
    encode_rgb_jpeg(fitted, jpeg_quality_for_max_side(max_side))
}

fn resize_jpeg_preview(bytes: &[u8], max_side: u32) -> Result<(Vec<u8>, &'static str), String> {
    use jpeg_decoder::{Decoder, PixelFormat};

    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.read_info().map_err(|e| e.to_string())?;
    let info = decoder
        .info()
        .ok_or_else(|| "jpeg metadata missing".to_string())?;
    let max_side_u16 = max_side.min(u16::MAX as u32) as u16;
    let (out_w, out_h) = decoder
        .scale(max_side_u16, max_side_u16)
        .map_err(|e| e.to_string())?;
    let pixels = decoder.decode().map_err(|e| e.to_string())?;

    let rgb = match info.pixel_format {
        PixelFormat::RGB24 => image::RgbImage::from_raw(out_w as u32, out_h as u32, pixels)
            .ok_or_else(|| "jpeg rgb buffer size mismatch".to_string())?,
        PixelFormat::L8 => {
            let mut rgb = image::RgbImage::new(out_w as u32, out_h as u32);
            for (i, &luma) in pixels.iter().enumerate() {
                let x = (i as u32) % out_w as u32;
                let y = (i as u32) / out_w as u32;
                rgb.put_pixel(x, y, image::Rgb([luma, luma, luma]));
            }
            rgb
        }
        _ => return Err("unsupported jpeg colorspace".into()),
    };

    let thumb = fit_rgb_to_max_side(rgb, max_side);
    encode_rgb_jpeg(thumb, jpeg_quality_for_max_side(max_side))
}

fn resize_jpeg_preview_thumb(bytes: &[u8], max_side: u32) -> Result<(Vec<u8>, &'static str), String> {
    resize_jpeg_preview(bytes, max_side)
}

fn resize_image_preview(bytes: &[u8], max_side: u32) -> Result<(Vec<u8>, &'static str), String> {
    if is_jpeg_bytes(bytes) {
        if let Ok(resized) = resize_jpeg_preview_thumb(bytes, max_side) {
            return Ok(resized);
        }
    }

    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    let fitted = fit_rgb_to_max_side(img.to_rgb8(), max_side);
    encode_rgb_jpeg(fitted, jpeg_quality_for_max_side(max_side))
}

fn preview_mime_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "ico" => "image/x-icon",
        "heic" | "heif" => "image/heic",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

#[tauri::command(rename = "backup_get_home_dir")]
pub fn get_home_dir() -> Option<String> {
    dirs::home_dir().map(|p| p.to_string_lossy().to_string())
}
