use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy)]
pub struct PartnerAppConfig {
    pub install_folder: &'static str,
    /// Feed SourceForge primario (`latest.json` Tauri o `latest.yml` Electron).
    pub sourceforge_latest_url: Option<&'static str>,
    /// Directory files SF usata per costruire URL download da `latest.yml` (trailing slash).
    pub sourceforge_files_base: Option<&'static str>,
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
    pub dev_electron: bool,
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
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct TauriPlatformAsset {
    url: String,
}

#[derive(Debug, Deserialize)]
struct TauriLatestManifest {
    version: String,
    #[serde(default)]
    platforms: std::collections::HashMap<String, TauriPlatformAsset>,
}

pub enum InstallKind {
    AppTarGz {
        url: String,
        name: String,
        digest: Option<String>,
    },
    LegacyBinary {
        url: String,
        name: String,
        digest: Option<String>,
    },
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    WindowsSetup {
        url: String,
        name: String,
        digest: Option<String>,
    },
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Dmg {
        url: String,
        name: String,
        digest: Option<String>,
    },
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
    if config.dev_electron {
        if path.join("package.json").is_file() {
            return Some(path);
        }
        return None;
    }
    if path.join("src-tauri").join("Cargo.toml").is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn dev_built_app(config: &PartnerAppConfig) -> Option<PathBuf> {
    let root = dev_project_root(config)?;
    if config.dev_electron {
        for dir in [
            "release/mac-arm64",
            "release/mac",
            "release/mac-universal",
            "release/mac-x64",
        ] {
            let app = root.join(dir).join(config.app_bundle_name);
            if app.is_dir() {
                return Some(app);
            }
        }
        return None;
    }
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

fn version_components(v: &str) -> Vec<u32> {
    let mut parts: Vec<u32> = normalize_version_tag(v)
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    while parts.len() > 1 && parts.last() == Some(&0) {
        parts.pop();
    }
    parts
}

pub fn version_gt(a: &str, b: &str) -> bool {
    let av = version_components(a);
    let bv = version_components(b);
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

fn is_legacy_install(
    config: &PartnerAppConfig,
    app: &AppHandle,
    opts: ResolveOptions,
) -> bool {
    #[cfg(target_os = "windows")]
    {
        if resolve_windows_exe(config, app).is_some() {
            return false;
        }
        return resolve_installed(config, app, opts).is_some();
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(path) = resolve_installed(config, app, opts) {
            return path.is_file();
        }
        return false;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (config, app, opts);
        false
    }
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

    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let Ok(uninstall) =
            RegKey::predef(hive).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall")
        else {
            continue;
        };

        for key_name in uninstall.enum_keys().flatten() {
            let Ok(key) = uninstall.open_subkey(&key_name) else {
                continue;
            };
            let display_name = key
                .get_value::<String, _>("DisplayName")
                .unwrap_or_default();
            let matches = key_name.eq_ignore_ascii_case(product_folder)
                || display_name.eq_ignore_ascii_case(product_folder)
                || display_name
                    .to_ascii_lowercase()
                    .contains(&product_folder.to_ascii_lowercase());
            if !matches {
                continue;
            }
            if let Ok(version) = key.get_value::<String, _>("DisplayVersion") {
                let version = version.trim();
                if !version.is_empty() {
                    return Some(normalize_version_tag(version));
                }
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

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("4uTools")
        .build()
        .map_err(|e| e.to_string())
}

fn fetch_text(url: &str) -> Result<String, String> {
    let client = http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("Impossibile contattare {url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Feed non trovato ({url}): {e}"))?;
    response
        .text()
        .map_err(|e| format!("Risposta non leggibile ({url}): {e}"))
}

fn asset_name_from_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches("/download");
    trimmed
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .to_string()
}

fn parse_tauri_latest_json(body: &str) -> Result<(String, Vec<ReleaseAsset>), String> {
    let manifest: TauriLatestManifest = serde_json::from_str(body)
        .map_err(|e| format!("Manifest Tauri non valido: {e}"))?;
    if manifest.version.trim().is_empty() {
        return Err("Manifest Tauri senza version".into());
    }
    let version = normalize_version_tag(&manifest.version);
    let assets = manifest
        .platforms
        .into_values()
        .map(|p| ReleaseAsset {
            name: asset_name_from_url(&p.url),
            browser_download_url: p.url,
            digest: None,
        })
        .collect::<Vec<_>>();
    if assets.is_empty() {
        return Err("Manifest Tauri senza piattaforme".into());
    }
    Ok((version, assets))
}

/// Minimal Electron `latest.yml` parser (version + files[].url / path).
fn parse_electron_latest_yml(
    body: &str,
    files_base: &str,
) -> Result<(String, Vec<ReleaseAsset>), String> {
    let mut version = None::<String>;
    let mut names = Vec::<String>::new();
    let mut in_files = false;

    for raw in body.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if indent == 0 {
            in_files = false;
            if let Some(rest) = trimmed.strip_prefix("version:") {
                version = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if trimmed == "files:" || trimmed.starts_with("files:") {
                in_files = true;
            } else if let Some(rest) = trimmed.strip_prefix("path:") {
                let name = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() && !names.iter().any(|n| n == &name) {
                    names.push(name);
                }
            }
            continue;
        }

        if in_files {
            if let Some(rest) = trimmed.strip_prefix("- url:") {
                let name = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() && !names.iter().any(|n| n == &name) {
                    names.push(name);
                }
            } else if let Some(rest) = trimmed.strip_prefix("url:") {
                let name = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() && !names.iter().any(|n| n == &name) {
                    names.push(name);
                }
            }
        }
    }

    let version = version.filter(|v| !v.is_empty()).ok_or_else(|| {
        "latest.yml senza version".to_string()
    })?;
    if names.is_empty() {
        return Err("latest.yml senza file".into());
    }

    let base = files_base.trim_end_matches('/');
    let assets = names
        .into_iter()
        .map(|name| ReleaseAsset {
            browser_download_url: format!("{base}/{name}/download"),
            name,
            digest: None,
        })
        .collect();
    Ok((normalize_version_tag(&version), assets))
}

fn fetch_sourceforge_latest(
    url: &str,
    files_base: Option<&str>,
) -> Result<(String, Vec<ReleaseAsset>), String> {
    let body = fetch_text(url)?;
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') {
        return parse_tauri_latest_json(&body);
    }
    let base = files_base.ok_or_else(|| {
        "sourceforge_files_base richiesto per feed latest.yml".to_string()
    })?;
    parse_electron_latest_yml(&body, base)
}

pub fn fetch_github_latest_release(url: &str) -> Result<(String, Vec<ReleaseAsset>), String> {
    let client = http_client()?;
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

/// SourceForge first (se configurato), altrimenti GitHub Releases API.
/// Se SF risponde ma senza asset installabili per la piattaforma corrente,
/// riusa la versione SF e prova gli asset da GitHub.
pub fn fetch_latest_release(
    config: &PartnerAppConfig,
) -> Result<(String, Vec<ReleaseAsset>), String> {
    let mut sf_err = None;
    if let Some(sf_url) = config.sourceforge_latest_url {
        match fetch_sourceforge_latest(sf_url, config.sourceforge_files_base) {
            Ok((ver, assets)) if has_platform_install_asset(&assets) => {
                return Ok((ver, assets));
            }
            Ok((ver, assets)) => {
                if let Ok((_, gh_assets)) = fetch_github_latest_release(config.github_latest_url) {
                    if has_platform_install_asset(&gh_assets) {
                        return Ok((ver, gh_assets));
                    }
                }
                return Ok((ver, assets));
            }
            Err(e) => sf_err = Some(e),
        }
    }
    match fetch_github_latest_release(config.github_latest_url) {
        Ok(release) => Ok(release),
        Err(gh_err) => Err(match sf_err {
            Some(sf) => {
                format!("SourceForge e GitHub non raggiungibili. SF: {sf}; GitHub: {gh_err}")
            }
            None => gh_err,
        }),
    }
}

pub fn fetch_release_assets(config: &PartnerAppConfig) -> Result<Vec<ReleaseAsset>, String> {
    Ok(fetch_latest_release(config)?.1)
}

fn has_platform_install_asset(assets: &[ReleaseAsset]) -> bool {
    #[cfg(target_os = "windows")]
    {
        return assets.iter().any(|a| {
            a.name.ends_with("-setup.exe")
                || a.name.ends_with("x64-setup.exe")
                || (a.name.ends_with(".exe") && !a.name.ends_with(".blockmap"))
        });
    }
    #[cfg(target_os = "macos")]
    {
        return assets.iter().any(|a| {
            a.name.ends_with(".app.tar.gz") || a.name.ends_with(".dmg")
        });
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = assets;
        false
    }
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

    let latest = fetch_latest_release(config).ok();
    let latest_version = latest.as_ref().map(|(v, _)| v.clone());

    let update_available = if !status.installed {
        latest.is_some()
    } else if let (Some(ref latest_v), Some(ref installed_v)) = (&latest_version, &installed_version)
    {
        version_gt(latest_v, installed_v)
    } else if is_legacy_install(config, app, opts) {
        latest
            .as_ref()
            .is_some_and(|(_, assets)| has_platform_install_asset(assets))
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

fn verify_file_digest(path: &Path, digest: Option<&str>) -> Result<(), String> {
    let Some(digest) = digest else {
        return Ok(());
    };
    let expected = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("Digest non supportato: {digest}"))?
        .to_ascii_lowercase();

    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let actual = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    if actual != expected {
        return Err("Verifica integrità download fallita (hash non corrispondente)".into());
    }
    Ok(())
}

pub fn download_file_with_progress(
    url: &str,
    dest: &Path,
    app: &AppHandle,
    app_id: &str,
    digest: Option<&str>,
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

    verify_file_digest(dest, digest)?;

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

#[cfg(target_os = "macos")]
pub fn install_macos_dmg(
    config: &PartnerAppConfig,
    app: &AppHandle,
    dmg_path: &Path,
) -> Result<PathBuf, String> {
    let dest = install_dir(config, app)?;
    let app_bundle = dest.join(config.app_bundle_name);
    if app_bundle.is_dir() {
        fs::remove_dir_all(&app_bundle).map_err(|e| e.to_string())?;
    }

    let dmg_str = dmg_path
        .to_str()
        .ok_or_else(|| "Percorso DMG non valido".to_string())?;

    let output = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", dmg_str])
        .output()
        .map_err(|e| format!("Mount DMG fallito: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Mount DMG fallito: {stderr}"));
    }

    let mount_point = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split('\t').last().map(str::trim))
        .filter(|p| Path::new(p).starts_with("/Volumes/"))
        .last()
        .ok_or_else(|| "Punto di mount DMG non trovato".to_string())?
        .to_string();

    let mount_path = PathBuf::from(&mount_point);
    let source = mount_path.join(config.app_bundle_name);
    let source = if source.is_dir() {
        source
    } else {
        fs::read_dir(&mount_path)
            .map_err(|e| format!("Lettura volume DMG fallita: {e}"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(".app"))
            })
            .ok_or_else(|| format!("{} non trovato nel DMG", config.app_bundle_name))?
    };

    let status = std::process::Command::new("ditto")
        .arg(&source)
        .arg(&app_bundle)
        .status()
        .map_err(|e| format!("Copia app fallita: {e}"))?;

    let _ = std::process::Command::new("hdiutil")
        .args(["detach", &mount_point])
        .status();

    if !status.success() {
        return Err("Copia app dal DMG fallita".into());
    }

    if !app_bundle.is_dir() {
        return Err(format!(
            "{} non trovato dopo l'installazione",
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

    for attempt in 0..12 {
        if let Some(path) = resolve_windows_exe(config, app) {
            return Ok(path);
        }
        if attempt < 11 {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
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
        InstallKind::AppTarGz { url, name, digest } => {
            let archive_path = dest_dir.join(&name);
            download_file_with_progress(&url, &archive_path, app, app_id, digest.as_deref())?;
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
        InstallKind::WindowsSetup { url, name, digest } => {
            let installer = dest_dir.join(&name);
            download_file_with_progress(&url, &installer, app, app_id, digest.as_deref())?;
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
        InstallKind::LegacyBinary { url, name, digest } => {
            let dest = dest_dir.join(&name);
            download_file_with_progress(&url, &dest, app, app_id, digest.as_deref())?;
            emit_progress(app, app_id, "install", 90.0);
            Ok(dest)
        }
        InstallKind::Dmg { url, name, digest } => {
            let dmg_path = dest_dir.join(&name);
            download_file_with_progress(&url, &dmg_path, app, app_id, digest.as_deref())?;
            emit_progress(app, app_id, "install", 85.0);
            #[cfg(target_os = "macos")]
            {
                let installed = install_macos_dmg(config, app, &dmg_path)?;
                let _ = fs::remove_file(&dmg_path);
                Ok(installed)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = dmg_path;
                Err("DMG supportato solo su macOS".into())
            }
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
    let npm_cmd = if config.dev_electron {
        "npm run dev"
    } else {
        "npm run tauri dev"
    };
    let cmd = format!("cd \"{root_str}\" && {npm_cmd}");
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
        if let Some(path) = resolve_installed(config, app, opts) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tauri_latest_json() {
        let body = r#"{
          "version": "1.2.3",
          "platforms": {
            "darwin-aarch64": {
              "url": "https://sourceforge.net/projects/autobkup/files/releases/AutoBackup_1.2.3_aarch64.app.tar.gz/download"
            },
            "windows-x86_64": {
              "url": "https://sourceforge.net/projects/autobkup/files/releases/AutoBackup_1.2.3_x64-setup.exe/download"
            }
          }
        }"#;
        let (ver, assets) = parse_tauri_latest_json(body).unwrap();
        assert_eq!(ver, "1.2.3");
        assert_eq!(assets.len(), 2);
        assert!(assets.iter().any(|a| a.name.ends_with("aarch64.app.tar.gz")));
        assert!(assets
            .iter()
            .any(|a| a.browser_download_url.ends_with("/download")));
    }

    #[test]
    fn parses_electron_latest_yml() {
        let body = r#"version: 0.9.1
files:
  - url: GoogleFotoManager-0.9.1-arm64-AppleSilicon.dmg
    sha512: abc
    size: 1
  - url: GoogleFotoManager-0.9.1-x64-Intel.dmg
    sha512: def
    size: 2
path: GoogleFotoManager-0.9.1-arm64-AppleSilicon.dmg
sha512: abc
releaseDate: '2026-01-01T00:00:00.000Z'
"#;
        let (ver, assets) = parse_electron_latest_yml(
            body,
            "https://sourceforge.net/projects/googlefotomanager/files/releases",
        )
        .unwrap();
        assert_eq!(ver, "0.9.1");
        assert_eq!(assets.len(), 2);
        assert_eq!(
            assets[0].browser_download_url,
            "https://sourceforge.net/projects/googlefotomanager/files/releases/GoogleFotoManager-0.9.1-arm64-AppleSilicon.dmg/download"
        );
    }

    #[test]
    fn asset_name_strips_download_suffix() {
        assert_eq!(
            asset_name_from_url(
                "https://sourceforge.net/projects/forutools/files/releases/v1.0.0/4uTools_1.0.0_x64-setup.exe/download"
            ),
            "4uTools_1.0.0_x64-setup.exe"
        );
    }
}
