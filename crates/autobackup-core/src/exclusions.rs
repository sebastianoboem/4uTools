use std::path::{Path, PathBuf};

pub fn system_folders() -> Vec<String> {
    if cfg!(target_os = "macos") {
        vec![
            "System".into(),
            "Library".into(),
            "Applications".into(),
            "private".into(),
            "usr".into(),
            "var".into(),
            ".Spotlight-V100".into(),
            ".fseventsd".into(),
            "Volumes".into(),
        ]
    } else if cfg!(windows) {
        vec![
            "Windows".into(),
            "Program Files".into(),
            "Program Files (x86)".into(),
            "ProgramData".into(),
            "$Recycle.Bin".into(),
            "System Volume Information".into(),
            "AppData".into(),
            "Boot".into(),
            "Recovery".into(),
            "PerfLogs".into(),
            "Config.Msi".into(),
            "Documents and Settings".into(),
            "MSOCache".into(),
            "$WinREAgent".into(),
            "OneDriveTemp".into(),
            "Temp".into(),
            "XboxGames".into(),
            "WindowsApps".into(),
            "Windows.old".into(),
        ]
    } else {
        vec![]
    }
}

pub fn android_system_folders() -> Vec<String> {
    vec![
        "/Android/data".into(),
        "/Android/obb".into(),
        "/.thumbnails".into(),
        "/.cache".into(),
        "/LOST.DIR".into(),
    ]
}

pub fn android_default_exceptions() -> Vec<String> {
    vec!["/sdcard/Android/media/com.whatsapp/WhatsApp/Media".into()]
}

fn is_hidden_relative(path: &Path, scan_root: Option<&Path>) -> bool {
    let rel = if let Some(root) = scan_root {
        let path_canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        path_canon
            .strip_prefix(&root_canon)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| {
                path.strip_prefix(root)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| path.to_path_buf())
            })
    } else {
        path.to_path_buf()
    };

    for part in rel.components() {
        if let Some(name) = part.as_os_str().to_str() {
            if name.starts_with('.') || name.starts_with('$') {
                return true;
            }
        }
    }

    #[cfg(windows)]
    {
        if scan_root.is_none() {
            if let Ok(metadata) = std::fs::metadata(path) {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
                if metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
                    return true;
                }
            }
        }
    }

    false
}

#[allow(dead_code)]
fn is_hidden_path(path: &Path) -> bool {
    is_hidden_relative(path, None)
}

pub fn is_excluded(path: &Path, exclusions: &[String], exceptions: &[String]) -> bool {
    is_excluded_with_root(path, None, exclusions, exceptions)
}

pub fn is_excluded_with_root(
    path: &Path,
    scan_root: Option<&Path>,
    exclusions: &[String],
    exceptions: &[String],
) -> bool {
    if is_hidden_relative(path, scan_root) {
        return true;
    }

    let path_str = path.to_string_lossy().to_lowercase();
    let mut excluded = false;

    for excl in exclusions {
        let excl_str = excl.to_lowercase();
        if path_str.starts_with(&excl_str) {
            excluded = true;
            break;
        }
        if !excl_str.contains(std::path::MAIN_SEPARATOR)
            && !excl_str.contains('/')
            && path
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .any(|part| part.eq_ignore_ascii_case(&excl_str))
        {
            excluded = true;
            break;
        }
    }

    if !excluded {
        return false;
    }

    for exc in exceptions {
        let exc_path = PathBuf::from(exc);
        if exc_path.starts_with(path) || path.starts_with(&exc_path) {
            return false;
        }
    }

    true
}

pub fn is_excluded_android(path: &str, exclusions: &[String], exceptions: &[String]) -> bool {
    let p = path.replace('\\', "/").to_lowercase();
    let excluded = exclusions.iter().any(|excl| {
        let e = excl.replace('\\', "/").to_lowercase();
        p.contains(&e)
    });

    if !excluded {
        return false;
    }

    for exc in exceptions {
        let exc = exc.replace('\\', "/").to_lowercase();
        if p.starts_with(&exc) {
            return false;
        }
    }

    true
}
