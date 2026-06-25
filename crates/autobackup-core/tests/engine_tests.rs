use autobackup_core::{
    apply_test_mode_limit, default_categories, get_removable_drives, is_excluded,
    resolve_backup_destination, scan_files, system_folders, BackupController, FileEntry,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn backup_destination_uses_subfolder_on_drive_root() {
    let path = resolve_backup_destination("/Volumes/USB", true, "Android_Backup");
    let s = path.to_string_lossy();
    assert!(s.starts_with("/Volumes/USB/Android_Backup_"));
}

#[test]
fn backup_destination_skips_subfolder_for_custom_folder() {
    let path = resolve_backup_destination("/Users/me/Backup", false, "Android_Backup");
    assert_eq!(path.to_string_lossy(), "/Users/me/Backup");
}

#[test]
fn categories_have_extensions() {
    let cats = default_categories();
    for (_name, exts) in cats {
        assert!(exts.iter().all(|e| e.starts_with('.')));
    }
}

#[test]
fn system_folders_not_empty_on_known_os() {
    let folders = system_folders();
    if cfg!(any(target_os = "macos", windows)) {
        assert!(!folders.is_empty());
    }
}

#[test]
fn scan_finds_matching_extension() {
    let tmp = TempDir::new().unwrap();
    let pdf = tmp.path().join("test.pdf");
    fs::write(&pdf, b"hello").unwrap();

    let mut map = HashMap::new();
    map.insert("Documenti".into(), vec![".pdf".into()]);

    let controller = BackupController::new();
    let (files, size) = scan_files(
        &[tmp.path().to_string_lossy().to_string()],
        &map,
        &[],
        &[],
        &[],
        &controller,
        None,
    );

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].category, "Documenti");
    assert!(size > 0);
}

#[test]
fn scan_excludes_hidden_folders() {
    let tmp = TempDir::new().unwrap();
    let hidden = tmp.path().join(".hidden");
    fs::create_dir(&hidden).unwrap();
    fs::write(hidden.join("secret.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("visible.pdf"), b"x").unwrap();

    let mut map = HashMap::new();
    map.insert("Documenti".into(), vec![".pdf".into()]);

    let controller = BackupController::new();
    let (files, _) = scan_files(
        &[tmp.path().to_string_lossy().to_string()],
        &map,
        &[],
        &[],
        &[],
        &controller,
        None,
    );

    let sources: Vec<_> = files.iter().map(|f| f.source.clone()).collect();
    assert!(sources.iter().any(|s| s.contains("visible.pdf")));
    assert!(!sources.iter().any(|s| s.contains("secret.pdf")));
}

#[test]
fn exclusion_hides_path() {
    let path = PathBuf::from("/Users/test/Windows/foo.txt");
    assert!(is_excluded(
        &path,
        &["Windows".into()],
        &[]
    ));
}

#[test]
fn test_mode_limits_per_category() {
    let files: Vec<FileEntry> = (0..15)
        .map(|i| FileEntry {
            source: format!("a{i}"),
            size: 1,
            category: "Documenti".into(),
            rel_path: format!("a{i}"),
            root_label: "Home".into(),
            device_id: None,
        })
        .collect();
    let (filtered, _) = apply_test_mode_limit(files, 10);
    assert_eq!(filtered.len(), 10);
}

#[test]
fn removable_drives_returns_vec() {
    let drives = get_removable_drives();
    let _ = drives;
}
