use crate::controller::BackupController;
use crate::process_util::hidden_command;
use crate::exclusions::{is_excluded_android, android_default_exceptions, android_system_folders};
use crate::models::{AndroidDevice, FileEntry, ScanProgress};
use crate::scanner::compile_extensions;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

pub fn find_adb() -> Option<String> {
    if let Some(path) = which_adb() {
        return Some(path);
    }
    let cwd_adb = if cfg!(windows) {
        Path::new("adb.exe")
    } else {
        Path::new("adb")
    };
    if cwd_adb.exists() {
        return Some(cwd_adb.to_string_lossy().to_string());
    }
    None
}

fn which_adb() -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = if cfg!(windows) {
                dir.join("adb.exe")
            } else {
                dir.join("adb")
            };
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
        None
    })
}

pub async fn get_devices(adb: &str) -> Vec<AndroidDevice> {
    let output = match hidden_command(adb)
        .args(["devices", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in text.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        if parts[1] == "offline" || parts[1] == "unauthorized" {
            continue;
        }
        let mut model = "Unknown".to_string();
        for p in &parts[2..] {
            if let Some(m) = p.strip_prefix("model:") {
                model = m.to_string();
            }
        }
        devices.push(AndroidDevice {
            id: parts[0].to_string(),
            model,
            status: parts[1].to_string(),
        });
    }

    devices
}

pub async fn scan_android_files(
    adb: &str,
    device_id: &str,
    category_extensions_map: &std::collections::HashMap<String, Vec<String>>,
    custom_extensions: &[String],
    exclusions: &[String],
    exceptions: &[String],
    controller: &BackupController,
    progress: Option<&crate::scanner::ScanCallback>,
) -> (Vec<FileEntry>, u64) {
    let (ext_to_cat, allowed_exts) =
        compile_extensions(category_extensions_map, custom_extensions);
    let allowed: std::collections::HashSet<String> =
        allowed_exts.iter().cloned().collect();

    let base_path = "/sdcard";
    let mut folders = list_android_folders(adb, device_id, base_path).await;
    if folders.is_empty() {
        folders = vec![
            format!("{base_path}/DCIM"),
            format!("{base_path}/Pictures"),
            format!("{base_path}/Download"),
            format!("{base_path}/Documents"),
            format!("{base_path}/Music"),
            format!("{base_path}/Movies"),
            format!("{base_path}/WhatsApp"),
            format!("{base_path}/Telegram"),
        ];
    }

    let mut files = Vec::new();
    let mut total_size = 0u64;

    for folder in folders {
        if controller.is_stopped() {
            break;
        }

        let stat_cmd = format!(
            "find '{folder}' \\( -path '*/.thumbnails' -o -path '*/.cache' -o -path '*/Android/data' \\) -prune -o -type f -exec stat -c '%s|%n' {{}} +"
        );

        let output = hidden_command(adb)
            .args(["-s", device_id, "shell", &stat_cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        let parsed = match output {
            Ok(o) if !o.stdout.is_empty() => parse_stat_output(&o.stdout, &allowed, exclusions, exceptions, &ext_to_cat, base_path, device_id),
            _ => {
                let find_cmd = format!(
                    "find '{folder}' \\( -path '*/.thumbnails' -o -path '*/.cache' -o -path '*/Android/data' \\) -prune -o -type f -print"
                );
                if let Ok(o) = hidden_command(adb)
                    .args(["-s", device_id, "shell", &find_cmd])
                    .stdout(Stdio::piped())
                    .output()
                    .await
                {
                    parse_find_output(&o.stdout, &allowed, exclusions, exceptions, &ext_to_cat, base_path, device_id)
                } else {
                    vec![]
                }
            }
        };

        for (entry, size) in parsed {
            total_size += size;
            if let Some(cb) = progress {
                cb(ScanProgress {
                    phase: "scanning".into(),
                    current_file: Path::new(&entry.source)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .into(),
                    files_found: files.len() + 1,
                });
            }
            files.push(entry);
        }
    }

    (files, total_size)
}

async fn list_android_folders(adb: &str, device_id: &str, base_path: &str) -> Vec<String> {
    let output = hidden_command(adb)
        .args(["-s", device_id, "shell", "ls", "-1", base_path])
        .stdout(Stdio::piped())
        .output()
        .await;

    let mut folders = Vec::new();
    if let Ok(o) = output {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let folder = line.trim();
            if folder.is_empty()
                || folder.starts_with('.')
                || matches!(folder, "lost+found" | ".thumbnails" | ".cache" | "Backups" | ".git")
            {
                continue;
            }
            folders.push(format!("{base_path}/{folder}"));
        }
    }
    folders
}

fn parse_stat_output(
    stdout: &[u8],
    allowed: &std::collections::HashSet<String>,
    exclusions: &[String],
    exceptions: &[String],
    ext_to_cat: &std::collections::HashMap<String, String>,
    base_path: &str,
    device_id: &str,
) -> Vec<(FileEntry, u64)> {
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(stdout).lines() {
        if !line.contains('|') {
            continue;
        }
        let Some((size_str, path)) = line.split_once('|') else {
            continue;
        };
        let Ok(size) = size_str.trim().parse::<u64>() else {
            continue;
        };
        let path = path.trim();
        if let Some(entry) = make_android_entry(path, size, allowed, exclusions, exceptions, ext_to_cat, base_path, device_id) {
            out.push((entry, size));
        }
    }
    out
}

fn parse_find_output(
    stdout: &[u8],
    allowed: &std::collections::HashSet<String>,
    exclusions: &[String],
    exceptions: &[String],
    ext_to_cat: &std::collections::HashMap<String, String>,
    base_path: &str,
    device_id: &str,
) -> Vec<(FileEntry, u64)> {
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(stdout).lines() {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }
        if let Some(entry) = make_android_entry(path, 0, allowed, exclusions, exceptions, ext_to_cat, base_path, device_id) {
            out.push((entry, 0));
        }
    }
    out
}

fn make_android_entry(
    path: &str,
    size: u64,
    allowed: &std::collections::HashSet<String>,
    exclusions: &[String],
    exceptions: &[String],
    ext_to_cat: &std::collections::HashMap<String, String>,
    base_path: &str,
    device_id: &str,
) -> Option<FileEntry> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))?;

    if !allowed.contains(&ext) {
        return None;
    }
    if is_excluded_android(path, exclusions, exceptions) {
        return None;
    }

    let rel_path = simplify_exception_path(path, base_path, exceptions);
    let category = ext_to_cat.get(&ext).cloned().unwrap_or_else(|| "Altro".into());

    Some(FileEntry {
        source: path.to_string(),
        size,
        category,
        rel_path,
        root_label: "Android".into(),
        device_id: Some(device_id.to_string()),
    })
}

fn simplify_exception_path(path: &str, base_path: &str, exceptions: &[String]) -> String {
    if exceptions.is_empty() {
        return strip_prefix_path(path, base_path);
    }

    let p = path.replace('\\', "/").to_lowercase();
    let mut best_exc: Option<&str> = None;
    let mut best_len = 0usize;

    for exc in exceptions {
        let e = exc.replace('\\', "/").to_lowercase();
        if p.starts_with(&e) && e.len() > best_len {
            best_len = e.len();
            best_exc = Some(exc.as_str());
        }
    }

    if let Some(best_exc) = best_exc {
        let mut rel = path[best_exc.len()..].to_string();
        if rel.starts_with('/') {
            rel.remove(0);
        }
        return rel;
    }

    strip_prefix_path(path, base_path)
}

fn strip_prefix_path(path: &str, base_path: &str) -> String {
    let path_norm = path.replace('\\', "/");
    let base_norm = base_path.trim_end_matches('/').replace('\\', "/");
    path_norm
        .strip_prefix(&base_norm)
        .map(|s| s.trim_start_matches('/').to_string())
        .unwrap_or(path_norm)
}

pub async fn copy_android_files(
    adb: &str,
    files: &[FileEntry],
    destination_root: &str,
    use_timestamp_subfolder: bool,
    controller: &BackupController,
    progress: Option<&crate::copier::CopyCallback>,
) -> (usize, Vec<String>, String) {
    let dest_path = crate::copier::resolve_backup_destination(
        destination_root,
        use_timestamp_subfolder,
        "Android_Backup",
    );
    if let Err(e) = std::fs::create_dir_all(&dest_path) {
        return (
            0,
            vec![format!("Cannot create destination: {e}")],
            String::new(),
        );
    }

    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let mut copied_count = 0usize;
    let mut copied_bytes = 0u64;
    let mut errors = Vec::new();

    for file_info in files {
        controller.wait_if_paused();
        if controller.is_stopped() {
            break;
        }

        let device_id = file_info.device_id.as_deref().unwrap_or("");
        let dst = dest_path.join(&file_info.rel_path);

        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Some(cb) = progress {
            cb(crate::models::CopyProgress {
                phase: "copying".into(),
                current_file: Path::new(&file_info.source)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .into(),
                copied_bytes,
                total_bytes,
                copied_count,
                total_count: files.len(),
            });
        }

        let status = hidden_command(adb)
            .args([
                "-s",
                device_id,
                "pull",
                &file_info.source,
                &dst.to_string_lossy(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => {
                copied_count += 1;
                copied_bytes += file_info.size;
            }
            Ok(s) => errors.push(format!(
                "Error pulling {}: exit {:?}",
                file_info.source, s.code()
            )),
            Err(e) => errors.push(format!("Error pulling {}: {e}", file_info.source)),
        }
    }

    (
        copied_count,
        errors,
        dest_path.to_string_lossy().to_string(),
    )
}

pub fn is_android_device_path(path: &str) -> bool {
    path.starts_with("/sdcard")
        || path.starts_with("/storage/")
        || path.starts_with("/data/")
}

pub async fn read_android_file_prefix(
    adb: &str,
    device_id: &str,
    remote_path: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let escaped = remote_path.replace('\'', "'\\''");
    let cmd = format!("head -c {max_bytes} '{escaped}'");
    let output = hidden_command(adb)
        .args(["-s", device_id, "exec-out", "sh", "-c", &cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("adb exec-out head failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Lettura parziale file Android fallita: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output.stdout)
}

pub async fn read_android_file(
    adb: &str,
    device_id: &str,
    remote_path: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let escaped = remote_path.replace('\'', "'\\''");
    let stat_cmd = format!("stat -c %s '{escaped}'");
    let stat_output = hidden_command(adb)
        .args(["-s", device_id, "shell", &stat_cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("adb stat failed: {e}"))?;

    if !stat_output.status.success() {
        return Err(format!(
            "Impossibile leggere dimensione file Android: {}",
            String::from_utf8_lossy(&stat_output.stderr)
        ));
    }

    let size_str = String::from_utf8_lossy(&stat_output.stdout).trim().to_string();
    let size: u64 = size_str
        .parse()
        .map_err(|_| format!("Dimensione non valida: {size_str}"))?;
    if size > max_bytes {
        return Err("File troppo grande per l'anteprima.".into());
    }

    let output = hidden_command(adb)
        .args(["-s", device_id, "exec-out", "cat", remote_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("adb exec-out failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Lettura file Android fallita: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output.stdout)
}

pub fn default_android_exclusions() -> Vec<String> {
    android_system_folders()
}

pub fn default_android_exceptions() -> Vec<String> {
    android_default_exceptions()
}

// Keep async helpers available for streaming if needed later
#[allow(dead_code)]
async fn read_lines(mut reader: BufReader<tokio::process::ChildStdout>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        lines.push(line.trim().to_string());
        line.clear();
    }
    lines
}
