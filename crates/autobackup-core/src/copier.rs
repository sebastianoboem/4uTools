use crate::controller::BackupController;
use crate::models::{CopyProgress, FileEntry};
use crate::verifier::verify_copy;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub type CopyCallback = Box<dyn Fn(CopyProgress) + Send + Sync>;

/// When `use_timestamp_subfolder` is true (destination is a drive root), creates
/// `{destination}/{prefix}_{unix_ts}`. Otherwise writes directly into `destination`.
pub fn resolve_backup_destination(
    destination_root: &str,
    use_timestamp_subfolder: bool,
    prefix: &str,
) -> PathBuf {
    let root = PathBuf::from(destination_root);
    if use_timestamp_subfolder {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        root.join(format!("{prefix}_{timestamp}"))
    } else {
        root
    }
}

pub fn copy_files(
    files: &[FileEntry],
    destination_root: &str,
    use_timestamp_subfolder: bool,
    verify: bool,
    controller: &BackupController,
    progress: Option<&CopyCallback>,
) -> (usize, Vec<String>, String) {
    let dest_path = resolve_backup_destination(destination_root, use_timestamp_subfolder, "Backup");
    if let Err(e) = fs::create_dir_all(&dest_path) {
        return (0, vec![format!("Cannot create destination: {e}")], String::new());
    }

    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let mut copied_count = 0usize;
    let mut copied_bytes = 0u64;
    let mut errors = Vec::new();
    let mut created_dirs = HashSet::new();
    created_dirs.insert(dest_path.clone());

    for file_info in files {
        controller.wait_if_paused();
        if controller.is_stopped() {
            break;
        }

        let src = Path::new(&file_info.source);
        let label = &file_info.root_label;
        let target_structure = Path::new(&file_info.rel_path);
        let dst = dest_path
            .join(&file_info.category)
            .join(label)
            .join(target_structure);

        if let Some(parent) = dst.parent() {
            if !created_dirs.contains(parent) {
                if let Err(e) = fs::create_dir_all(parent) {
                    errors.push(format!("Error creating dir {}: {e}", parent.display()));
                    continue;
                }
                created_dirs.insert(parent.to_path_buf());
            }
        }

        if let Some(cb) = progress {
            cb(CopyProgress {
                phase: "copying".into(),
                current_file: src
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

        match fs::copy(src, &dst) {
            Ok(_) => {
                if verify {
                    match verify_copy(src, &dst) {
                        Ok(true) => {}
                        Ok(false) => errors.push(format!("Integrity check failed: {}", src.display())),
                        Err(e) => errors.push(format!("Verify error for {}: {e}", src.display())),
                    }
                }
                copied_count += 1;
                copied_bytes += file_info.size;
            }
            Err(e) => errors.push(format!("Error copying {}: {e}", src.display())),
        }
    }

    (
        copied_count,
        errors,
        dest_path.to_string_lossy().to_string(),
    )
}

pub fn apply_test_mode_limit(files: Vec<FileEntry>, per_category: usize) -> (Vec<FileEntry>, u64) {
    let mut counts = std::collections::HashMap::new();
    let mut filtered = Vec::new();
    let mut size = 0u64;

    for f in files {
        let count = counts.entry(f.category.clone()).or_insert(0usize);
        if *count < per_category {
            size += f.size;
            filtered.push(f);
            *count += 1;
        }
    }

    (filtered, size)
}
