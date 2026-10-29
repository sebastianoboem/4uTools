use std::path::Path;

/// Copy data written by the standalone apps into the 4uTools data dir once.
/// AutoBackup and PriceCreator keep settings in memory, so only json/sqlite
/// files found under their old data dirs are copied.
pub fn import_legacy() {
    let Some(data) = dirs::data_dir() else {
        return;
    };
    let dest = data.join("4uTools");
    let flag = dest.join(".legacy-import-done");
    if flag.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dest);

    let adware = data.join("AndroidAdwareCleaner");
    for name in ["reputation.db", "config.json", "supabase-session.json"] {
        copy_if_absent(&adware.join(name), &dest.join(name));
    }
    copy_tree(&adware.join("platform-tools"), &dest.join("platform-tools"));

    for id in [
        "com.ilpano.android-adware-cleaner",
        "com.autobackup.desktop",
        "com.doctorphone.pricecreator",
    ] {
        copy_data_files(&data.join(id), &dest.join("imported").join(id));
    }

    let _ = std::fs::write(flag, b"1\n");
}

/// Copy Google Foto Manager profiles into 4uTools. Separate flag: the phase-1
/// import may already have run before this tab existed.
pub fn import_gfm() {
    let Some(data) = dirs::data_dir() else {
        return;
    };
    let dest = data.join("4uTools").join("gphotos");
    let flag = data.join("4uTools").join(".gfm-import-done");
    if flag.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dest);
    let src = data.join("Google Foto Manager");
    for name in ["settings.json", "app-settings.json", "profiles.json"] {
        copy_if_absent(&src.join(name), &dest.join(name));
    }
    copy_tree(&src.join("profile-data"), &dest.join("profile-data"));
    let _ = std::fs::write(flag, b"1\n");
}

fn copy_if_absent(src: &Path, dest: &Path) {
    if dest.exists() || !src.is_file() {
        return;
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::copy(src, dest);
}

fn copy_tree(src: &Path, dest: &Path) {
    let Ok(entries) = std::fs::read_dir(src) else {
        return;
    };
    let _ = std::fs::create_dir_all(dest);
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            copy_if_absent(&path, &target);
        }
    }
}

fn copy_data_files(src: &Path, dest: &Path) {
    let Ok(entries) = std::fs::read_dir(src) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_dir() {
            if matches!(
                name_str.as_ref(),
                "Cache" | "GPUCache" | "Code Cache" | "blob_storage"
            ) {
                continue;
            }
            copy_data_files(&path, &dest.join(name));
            continue;
        }
        let keep = name_str.ends_with(".json")
            || name_str.ends_with(".db")
            || name_str.ends_with(".sqlite")
            || name_str.ends_with(".sqlite3");
        if keep {
            copy_if_absent(&path, &dest.join(name));
        }
    }
}
