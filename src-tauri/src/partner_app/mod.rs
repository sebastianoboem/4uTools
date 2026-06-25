use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy)]
pub struct PartnerAppConfig {
    pub install_folder: &'static str,
    pub github_latest_url: &'static str,
    pub app_bundle_name: &'static str,
    pub dev_env_var: &'static str,
    pub dev_default_path: Option<&'static str>,
    pub legacy_mac_binary: Option<&'static str>,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub legacy_win_binary: Option<&'static str>,
    /// Basenames (no `.exe`) tried under typical Tauri NSIS install folders.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub windows_exe_basenames: &'static [&'static str],
    pub not_installed_error: &'static str,
}

#[derive(Debug, Serialize)]
pub struct PartnerStatus {
    pub installed: bool,
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartnerUpdateStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub update_available: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct PartnerInstallProgress {
    pub app_id: String,
    pub phase: String,
    pub percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

pub enum InstallKind {
    AppTarGz { url: String, name: String },
    LegacyBinary { url: String, name: String },
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    WindowsSetup { url: String, name: String },
}

#[derive(Clone, Copy)]
pub struct ResolveOptions {
    pub allow_files: bool,
}

pub fn install_dir(config: &PartnerAppConfig, app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join(config.install_folder);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn dev_project_root(config: &PartnerAppConfig) -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let path = std::env::var(config.dev_env_var)
        .ok()
        .map(PathBuf::from)
        .or_else(|| config.dev_default_path.map(PathBuf::from))?;
    if path.join("src-tauri").join("Cargo.toml").is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn dev_built_app(config: &PartnerAppConfig) -> Option<PathBuf> {
    let root = dev_project_root(config)?;
    for profile in ["debug", "release"] {
        for base in ["target", "src-tauri/target"] {
            let app = root.join(format!(
                "{base}/{profile}/bundle/macos/{}",
                config.app_bundle_name
            ));
            if app.is_dir() {
                return Some(app);
            }
        }
    }
    None
}

pub fn installed_candidates(
    config: &PartnerAppConfig,
    app: &AppHandle,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = vec![install_dir(config, app)?.join(config.app_bundle_name)];

    if let Some(name) = config.legacy_mac_binary {
        paths.push(install_dir(config, app)?.join(name));
    }

    paths.push(
        PathBuf::from("/Applications").join(config.app_bundle_name),
    );

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Applications").join(config.app_bundle_name));
    }

    Ok(paths)
}

pub fn resolve_installed(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> Option<PathBuf> {
    if let Some(app) = dev_built_app(config) {
        return Some(app);
    }
    installed_candidates(config, app)
        .ok()?
        .into_iter()
        .find(|p| {
            if p.is_dir() {
                return true;
            }
            opts.allow_files && p.is_file()
        })
}

#[cfg(target_os = "windows")]
fn windows_install_roots(install_folder: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        roots.push(local.join(install_folder));
        roots.push(local.join("Programs").join(install_folder));
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(pf) = std::env::var(var) {
            roots.push(PathBuf::from(pf).join(install_folder));
        }
    }
    roots
}

#[cfg(target_os = "windows")]
fn is_uninstaller_exe(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|stem| {
            let lower = stem.to_lowercase();
            lower.contains("uninst") || lower == "uninstall"
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn find_exe_in_dir(dir: &Path, basenames: &[&str]) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    for base in basenames {
        let exe = dir.join(format!("{base}.exe"));
        if exe.is_file() {
            return Some(exe);
        }
    }

    let mut fallback = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("exe") || is_uninstaller_exe(&path) {
                continue;
            }
            if basenames.iter().any(|base| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|stem| stem.eq_ignore_ascii_case(base))
            }) {
                return Some(path);
            }
            if fallback.is_none() {
                fallback = Some(path);
            }
        }
    }

    fallback
}

#[cfg(target_os = "windows")]
fn registry_install_exe(product_folder: &str, basenames: &[&str]) -> Option<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let subkey = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product_folder}");

    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let Ok(key) = RegKey::predef(hive).open_subkey(&subkey) else {
            continue;
        };

        if let Ok(main_binary) = key.get_value::<String, _>("MainBinaryName") {
            if let Ok(loc) = key.get_value::<String, _>("InstallLocation") {
                let install_dir = PathBuf::from(loc.trim().trim_matches('"'));
                let exe = install_dir.join(main_binary.trim());
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }

        if let Ok(loc) = key.get_value::<String, _>("InstallLocation") {
            let install_dir = PathBuf::from(loc.trim().trim_matches('"'));
            if let Some(exe) = find_exe_in_dir(&install_dir, basenames) {
                return Some(exe);
            }
        }

        if let Ok(icon) = key.get_value::<String, _>("DisplayIcon") {
            let path_str = icon.split(',').next()?.trim().trim_matches('"');
            let path = PathBuf::from(path_str);
            if path.is_file() && !is_uninstaller_exe(&path) {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
pub fn resolve_windows_exe(config: &PartnerAppConfig, app: &AppHandle) -> Option<PathBuf> {
    if let Some(path) = registry_install_exe(config.install_folder, config.windows_exe_basenames) {
        return Some(path);
    }

    for root in windows_install_roots(config.install_folder) {
        if let Some(path) = find_exe_in_dir(&root, config.windows_exe_basenames) {
            return Some(path);
        }
    }

    if let Ok(dir) = install_dir(config, app) {
        if let Some(name) = config.legacy_win_binary {
            let legacy = dir.join(name);
            if legacy.is_file() {
                return Some(legacy);
            }
        }
        if let Some(path) = find_exe_in_dir(&dir, config.windows_exe_basenames) {
            return Some(path);
        }
    }

    None
}

pub fn check_installed(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> PartnerStatus {
    if dev_project_root(config).is_some() {
        return PartnerStatus {
            installed: true,
            path: resolve_installed(config, app, opts).map(|p| p.to_string_lossy().into_owned()),
        };
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(path) = resolve_windows_exe(config, app) {
            return PartnerStatus {
                installed: true,
                path: Some(path.to_string_lossy().into_owned()),
            };
        }
    }

    if let Some(path) = resolve_installed(config, app, opts) {
        return PartnerStatus {
            installed: true,
            path: Some(path.to_string_lossy().into_owned()),
        };
    }

    PartnerStatus {
        installed: false,
        path: None,
    }
}

pub fn normalize_version_tag(tag: &str) -> String {
    tag.trim().trim_start_matches('v').to_string()
}

pub fn version_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        normalize_version_tag(v)
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let av = parse(a);
    let bv = parse(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        if ai != bi {
            return ai > bi;
        }
    }
    false
}

fn read_plist_version(text: &str) -> Option<String> {
    let key = "CFBundleShortVersionString</key>";
    let after_key = text.find(key).map(|i| &text[i + key.len()..])?;
    let string_open = after_key.find("<string>")? + "<string>".len();
    let rest = &after_key[string_open..];
    let end = rest.find("</string>")?;
    Some(rest[..end].trim().to_string())
}

#[cfg(target_os = "macos")]
fn read_macos_bundle_version(path: &Path) -> Option<String> {
    let plist = path.join("Contents/Info.plist");
    let text = fs::read_to_string(plist).ok()?;
    read_plist_version(&text)
}

#[cfg(target_os = "windows")]
fn read_windows_registry_version(product_folder: &str) -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let subkey = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product_folder}");
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let Ok(key) = RegKey::predef(hive).open_subkey(&subkey) else {
            continue;
        };
        if let Ok(version) = key.get_value::<String, _>("DisplayVersion") {
            if !version.trim().is_empty() {
                return Some(version);
            }
        }
    }
    None
}

pub fn installed_version(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(path) = resolve_installed(config, app, opts) {
            if path.is_dir() {
                return read_macos_bundle_version(&path);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        return read_windows_registry_version(config.install_folder);
    }

    None
}

pub fn fetch_latest_release(url: &str) -> Result<(String, Vec<ReleaseAsset>), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("4uTools")
        .build()
        .map_err(|e| e.to_string())?;
    let release: ReleaseInfo = client
        .get(url)
        .send()
        .map_err(|e| format!("Impossibile contattare GitHub: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Release non trovata: {e}"))?
        .json()
        .map_err(|e| format!("Risposta GitHub non valida: {e}"))?;
    Ok((normalize_version_tag(&release.tag_name), release.assets))
}

pub fn fetch_release_assets(url: &str) -> Result<Vec<ReleaseAsset>, String> {
    Ok(fetch_latest_release(url)?.1)
}

pub fn check_update_status(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> PartnerUpdateStatus {
    let status = check_installed(config, app, opts);
    let installed_version = if status.installed {
        installed_version(config, app, opts)
    } else {
        None
    };

    let latest = fetch_latest_release(config.github_latest_url).ok();
    let latest_version = latest.as_ref().map(|(v, _)| v.clone());

    let update_available = if !status.installed {
        latest.is_some()
    } else if let (Some(ref latest_v), Some(ref installed_v)) = (&latest_version, &installed_version)
    {
        version_gt(latest_v, installed_v)
    } else {
        false
    };

    PartnerUpdateStatus {
        installed: status.installed,
        path: status.path,
        update_available,
        installed_version,
        latest_version,
    }
}

fn emit_progress(app: &AppHandle, app_id: &str, phase: &str, percent: f64) {
    let _ = app.emit(
        "partner-install-progress",
        PartnerInstallProgress {
            app_id: app_id.to_string(),
            phase: phase.to_string(),
            percent: percent.clamp(0.0, 100.0),
        },
    );
}

pub fn download_file_with_progress(
    url: &str,
    dest: &Path,
    app: &AppHandle,
    app_id: &str,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("4uTools")
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("Download fallito: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download fallito: {e}"))?;

    let total = response.content_length();
    let tmp = dest.with_extension("download");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|e| format!("Download fallito: {e}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("Download fallito: {e}"))?;
        downloaded += read as u64;
        let pct = match total {
            Some(total) if total > 0 => (downloaded as f64 / total as f64) * 80.0,
            _ => 40.0,
        };
        emit_progress(app, app_id, "download", pct);
    }

    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    emit_progress(app, app_id, "download", 80.0);

    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(dest).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(dest).map_err(|e| e.to_string())?;
        }
    }
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    if dest.is_file() {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn install_macos_app(
    config: &PartnerAppConfig,
    app: &AppHandle,
    archive: &Path,
) -> Result<PathBuf, String> {
    let dest = install_dir(config, app)?;
    let app_bundle = dest.join(config.app_bundle_name);
    if app_bundle.is_dir() {
        fs::remove_dir_all(&app_bundle).map_err(|e| e.to_string())?;
    }

    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(&dest)
        .status()
        .map_err(|e| format!("Estrazione fallita: {e}"))?;

    if !status.success() {
        return Err("Estrazione archivio .app fallita".into());
    }

    if !app_bundle.is_dir() {
        return Err(format!(
            "{} non trovato dopo l'estrazione",
            config.app_bundle_name
        ));
    }

    Ok(app_bundle)
}

#[cfg(target_os = "windows")]
pub fn install_windows_setup(
    config: &PartnerAppConfig,
    app: &AppHandle,
    installer: &Path,
    restart_hint: &str,
) -> Result<PathBuf, String> {
    let status = std::process::Command::new(installer)
        .arg("/S")
        .status()
        .map_err(|e| format!("Installazione fallita: {e}"))?;

    if !status.success() {
        return Err("Installazione silenziosa fallita".into());
    }

    resolve_windows_exe(config, app).ok_or_else(|| restart_hint.to_string())
}

pub fn install_from_kind_with_progress(
    config: &PartnerAppConfig,
    app: &AppHandle,
    kind: InstallKind,
    _restart_hint: &str,
    app_id: &str,
) -> Result<PathBuf, String> {
    let dest_dir = install_dir(config, app)?;

    let result = match kind {
        InstallKind::AppTarGz { url, name } => {
            let archive_path = dest_dir.join(name);
            download_file_with_progress(&url, &archive_path, app, app_id)?;
            emit_progress(app, app_id, "install", 85.0);
            #[cfg(target_os = "macos")]
            {
                let installed = install_macos_app(config, app, &archive_path)?;
                let _ = fs::remove_file(&archive_path);
                Ok(installed)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = archive_path;
                Err("Archivio .app supportato solo su macOS".into())
            }
        }
        InstallKind::WindowsSetup { url, name } => {
            let installer = dest_dir.join(name);
            download_file_with_progress(&url, &installer, app, app_id)?;
            emit_progress(app, app_id, "install", 85.0);
            #[cfg(target_os = "windows")]
            {
                let installed = install_windows_setup(config, app, &installer, _restart_hint)?;
                let _ = fs::remove_file(&installer);
                Ok(installed)
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = installer;
                Err("Setup Windows supportato solo su Windows".into())
            }
        }
        InstallKind::LegacyBinary { url, name } => {
            let dest = dest_dir.join(name);
            download_file_with_progress(&url, &dest, app, app_id)?;
            emit_progress(app, app_id, "install", 90.0);
            Ok(dest)
        }
    };

    if result.is_ok() {
        emit_progress(app, app_id, "install", 100.0);
    }
    result
}

#[cfg(target_os = "macos")]
fn launch_legacy_cli(path: &Path, label: &str) -> Result<(), String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| format!("Percorso {label} non valido"))?;
    let script = format!(
        "tell application \"Terminal\" to do script \"{}\"",
        path_str.replace('\\', "\\\\").replace('"', "\\\"")
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .map_err(|e| format!("Impossibile avviare {label}: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn launch_dev_project(config: &PartnerAppConfig, label: &str) -> Result<(), String> {
    let root = dev_project_root(config).ok_or_else(|| "Progetto dev non trovato".to_string())?;
    let root_str = root
        .to_str()
        .ok_or_else(|| "Percorso dev non valido".to_string())?;
    let cmd = format!("cd \"{root_str}\" && npm run tauri dev");
    let script = format!(
        "tell application \"Terminal\" to do script \"{}\"",
        cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .map_err(|e| format!("Impossibile avviare {label} (dev): {e}"))?;
    Ok(())
}

pub fn launch_path(config: &PartnerAppConfig, path: &Path) -> Result<(), String> {
    let label = config.install_folder;

    if path.extension().is_none() && path.is_file() {
        #[cfg(target_os = "macos")]
        {
            return launch_legacy_cli(path, label);
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Impossibile avviare {label}: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(path)
            .spawn()
            .map_err(|e| format!("Impossibile avviare {label}: {e}"))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        Err("Piattaforma non supportata".into())
    }
}

pub fn launch_installed(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = resolve_windows_exe(config, app) {
            return launch_path(config, &path);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) = resolve_installed(config, app, opts) {
            return launch_path(config, &path);
        }
        if cfg!(debug_assertions) && dev_project_root(config).is_some() {
            #[cfg(target_os = "macos")]
            {
                return launch_dev_project(config, config.install_folder);
            }
        }
    }

    Err(config.not_installed_error.to_string())
}

pub fn resolve_path_or_error(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
    already_installed_msg: &str,
) -> Result<PathBuf, String> {
    resolve_installed(config, app, opts)
        .or_else(|| {
            #[cfg(target_os = "windows")]
            {
                resolve_windows_exe(config, app)
            }
            #[cfg(not(target_os = "windows"))]
            {
                None
            }
        })
        .ok_or_else(|| already_installed_msg.to_string())
}
