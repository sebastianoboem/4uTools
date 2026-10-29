use crate::controller::BackupController;
use crate::exclusions::is_excluded_with_root;
use crate::models::{FileEntry, ScanProgress};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub type ScanCallback = Box<dyn Fn(ScanProgress) + Send + Sync>;

pub fn compile_extensions(
    category_extensions_map: &HashMap<String, Vec<String>>,
    custom_extensions: &[String],
) -> (HashMap<String, String>, Vec<String>) {
    let mut ext_to_cat = HashMap::new();
    for (cat, exts) in category_extensions_map {
        for e in exts {
            let lower = e.to_lowercase();
            ext_to_cat.insert(lower.clone(), cat.clone());
        }
    }
    for e in custom_extensions {
        let lower = e.to_lowercase();
        ext_to_cat.entry(lower.clone()).or_insert_with(|| "Altro".into());
    }
    let allowed: Vec<String> = ext_to_cat.keys().cloned().collect();
    (ext_to_cat, allowed)
}

pub fn root_label_for(drive_path: &Path) -> String {
    if cfg!(target_os = "macos") {
        let p = drive_path.to_string_lossy();
        if p.starts_with("/Volumes/") {
            return drive_path
                .components()
                .nth(2)
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or("Volume")
                .to_string();
        }
        if let Some(home) = dirs::home_dir() {
            if drive_path == home || drive_path.starts_with(&home) {
                return "Home".into();
            }
        }
        String::new()
    } else if cfg!(windows) {
        drive_path
            .to_string_lossy()
            .chars()
            .take_while(|c| *c != ':')
            .collect::<String>()
    } else {
        String::new()
    }
}

fn start_paths_for_drive(drive_path: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(system_drive) = std::env::var("SystemDrive") {
            let system_drive = system_drive.to_uppercase();
            let current_root = drive_path
                .to_string_lossy()
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase();

            if current_root == system_drive {
                let abs_input = drive_path
                    .canonicalize()
                    .unwrap_or_else(|_| drive_path.to_path_buf());
                let abs_root = PathBuf::from(&system_drive);
                if abs_input == abs_root || abs_input == PathBuf::from(format!("{system_drive}\\")) {
                    if let Some(home) = dirs::home_dir() {
                        if home
                            .to_string_lossy()
                            .to_uppercase()
                            .starts_with(&current_root)
                        {
                            return vec![home];
                        }
                    }
                    let users = drive_path.join("Users");
                    if users.exists() {
                        return vec![users];
                    }
                }
            }
        }
    }
    vec![drive_path.to_path_buf()]
}

pub fn scan_files(
    source_roots: &[String],
    category_extensions_map: &HashMap<String, Vec<String>>,
    custom_extensions: &[String],
    exclusions: &[String],
    exceptions: &[String],
    controller: &BackupController,
    progress: Option<&ScanCallback>,
) -> (Vec<FileEntry>, u64) {
    let (ext_to_cat, allowed_exts) =
        compile_extensions(category_extensions_map, custom_extensions);
    let allowed_set: std::collections::HashSet<&str> =
        allowed_exts.iter().map(|s| s.as_str()).collect();

    let mut files = Vec::new();
    let mut total_size = 0u64;

    for source in source_roots {
        if controller.is_stopped() {
            return (vec![], 0);
        }

        let drive_path = PathBuf::from(source);
        let label = root_label_for(&drive_path);
        let start_paths = start_paths_for_drive(&drive_path);

        for start_path in start_paths {
            let start_canon = start_path.canonicalize().unwrap_or_else(|_| start_path.clone());

            for entry in WalkDir::new(&start_path).follow_links(false).into_iter().filter_entry(|e| {
                let p = e.path();
                let p_canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                if p == start_path.as_path() || p_canon == start_canon {
                    return true;
                }
                !is_excluded_with_root(p, Some(&start_canon), exclusions, exceptions)
            }) {
                if controller.is_stopped() {
                    return (vec![], 0);
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let file_path = entry.path();
                if is_excluded_with_root(file_path, Some(&start_canon), exclusions, exceptions) {
                    continue;
                }

                let ext = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{}", e.to_lowercase()))
                    .unwrap_or_default();

                if !allowed_set.contains(ext.as_str()) {
                    continue;
                }

                let metadata = match file_path.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let rel_path = file_path
                    .strip_prefix(&drive_path)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();

                let category = ext_to_cat
                    .get(&ext)
                    .cloned()
                    .unwrap_or_else(|| "Altro".into());

                files.push(FileEntry {
                    source: file_path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    category,
                    rel_path,
                    root_label: label.clone(),
                    device_id: None,
                });
                total_size += metadata.len();

                if let Some(cb) = progress {
                    cb(ScanProgress {
                        phase: "scanning".into(),
                        current_file: file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .into(),
                        files_found: files.len(),
                    });
                }
            }
        }
    }

    (files, total_size)
}

pub fn resolve_scan_roots(
    whitelist_paths: &[String],
    destination_mount: &str,
) -> Vec<String> {
    let _ = destination_mount;
    #[cfg(windows)]
    {
        return resolve_scan_roots_windows(whitelist_paths, destination_mount);
    }

    #[cfg(target_os = "macos")]
    {
        if whitelist_paths.is_empty() {
            if let Some(home) = dirs::home_dir() {
                return vec![home.to_string_lossy().to_string()];
            }
        }
        return whitelist_paths.to_vec();
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        whitelist_paths.to_vec()
    }
}

#[cfg(windows)]
fn resolve_scan_roots_windows(whitelist_paths: &[String], destination_mount: &str) -> Vec<String> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let mut fixed_drives: Vec<String> = disks
        .list()
        .iter()
        .filter(|d| !d.is_removable())
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect();

    let dest_norm = destination_mount.trim_end_matches('\\').to_lowercase();
    fixed_drives.retain(|d| d.trim_end_matches('\\').to_lowercase() != dest_norm);

    let mut all_roots = Vec::new();
    let mut used_whitelist = Vec::new();

    for drive in &fixed_drives {
        let drive_specific: Vec<String> = whitelist_paths
            .iter()
            .filter(|wp| is_on_drive(wp, drive))
            .cloned()
            .collect();

        if !drive_specific.is_empty() {
            all_roots.extend(drive_specific.clone());
            used_whitelist.extend(drive_specific);
        } else if whitelist_paths.is_empty() {
            all_roots.push(drive.clone());
        } else {
            all_roots.push(drive.clone());
        }
    }

    for wp in whitelist_paths {
        if !used_whitelist.contains(wp) {
            all_roots.push(wp.clone());
        }
    }

    if all_roots.is_empty() && !whitelist_paths.is_empty() {
        return whitelist_paths.to_vec();
    }

    all_roots
}

#[cfg(windows)]
fn is_on_drive(path: &str, drive_root: &str) -> bool {
    let p = PathBuf::from(path)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_else(|_| path.to_lowercase());
    let mut d = drive_root.to_lowercase();
    if !d.ends_with('\\') {
        d.push('\\');
    }
    p.starts_with(&d)
}
