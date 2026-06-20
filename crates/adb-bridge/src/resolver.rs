use std::path::PathBuf;

/// Resolve adb binary: custom path → app install dir → bundled resources → PATH → ANDROID_HOME.
pub fn resolve_adb_path() -> Option<PathBuf> {
    if let Some(p) = custom_adb() {
        return Some(p);
    }

    if let Some(p) = app_installed_adb() {
        return Some(p);
    }

    if let Some(p) = bundled_adb() {
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(path) = which_adb() {
        return Some(path);
    }

    if let Ok(home) = std::env::var("ANDROID_HOME").or_else(|_| std::env::var("ANDROID_SDK_ROOT")) {
        let candidate = PathBuf::from(home).join("platform-tools").join(adb_exe_name());
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn custom_adb() -> Option<PathBuf> {
    let raw = std::env::var("FOURUTOOLS_CUSTOM_ADB")
        .or_else(|_| std::env::var("ANDROID_ADWARE_CUSTOM_ADB"))
        .ok()?;
    let path = PathBuf::from(&raw);
    if path.is_file() {
        return Some(path);
    }
    let exe = adb_exe_name();
    let as_platform_tools = path.join("platform-tools").join(exe);
    if as_platform_tools.exists() {
        return Some(as_platform_tools);
    }
    let direct = path.join(exe);
    if direct.exists() {
        return Some(direct);
    }
    None
}

fn app_installed_adb() -> Option<PathBuf> {
    let exe = adb_exe_name();
    if let Ok(dir) = std::env::var("FOURUTOOLS_PLATFORM_TOOLS")
        .or_else(|_| std::env::var("ANDROID_ADWARE_PLATFORM_TOOLS"))
    {
        let p = PathBuf::from(&dir).join("platform-tools").join(exe);
        if p.exists() {
            return Some(p);
        }
        let direct = PathBuf::from(&dir).join(exe);
        if direct.exists() {
            return Some(direct);
        }
    }
    if let Some(data) = dirs::data_dir() {
        for app_name in ["4uTools", "AndroidAdwareCleaner"] {
            let p = data
                .join(app_name)
                .join("platform-tools")
                .join("platform-tools")
                .join(exe);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn adb_exe_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}

fn bundled_adb() -> Option<PathBuf> {
    let exe = adb_exe_name();
    let candidates = [
        PathBuf::from("resources/platform-tools").join(exe),
        PathBuf::from("../resources/platform-tools").join(exe),
    ];

    if let Ok(cwd) = std::env::current_dir() {
        for rel in candidates {
            let p = cwd.join(&rel);
            if p.exists() {
                return Some(p);
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for ancestor in parent.ancestors().take(5) {
                let p = ancestor.join("resources/platform-tools").join(exe);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    None
}

fn which_adb() -> Option<PathBuf> {
    let name = adb_exe_name();
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
