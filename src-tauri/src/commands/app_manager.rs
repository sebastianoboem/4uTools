use crate::partner_app::{
    check_update_status, fetch_release_assets, install_from_kind_with_progress, launch_installed,
    resolve_path_or_error, InstallKind, PartnerAppConfig, PartnerUpdateStatus, ResolveOptions,
};
use tauri::AppHandle;

const APP_ID: &str = "app_manager";

const CONFIG: PartnerAppConfig = PartnerAppConfig {
    install_folder: "AndroidAdwareCleaner",
    github_latest_url:
        "https://api.github.com/repos/sebastianoboem/AndroidAdwareCleaner/releases/latest",
    app_bundle_name: "AndroidAdwareCleaner.app",
    dev_env_var: "ANDROID_ADWARE_CLEANER_DEV",
    dev_default_path: Some("/Users/ilpano/Projects/AndroidAdwareCleaner"),
    legacy_mac_binary: None,
    legacy_win_binary: None,
    windows_exe_basenames: &["AndroidAdwareCleaner", "android-adware-cleaner"],
    not_installed_error: "AndroidAdwareCleaner non è installato",
};

const RESOLVE_OPTS: ResolveOptions = ResolveOptions {
    allow_files: false,
};

pub type AppManagerStatus = PartnerUpdateStatus;

fn asset_suffix() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "aarch64.app.tar.gz";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "x64.app.tar.gz";
    }
    #[cfg(target_os = "windows")]
    {
        return "x64-setup.exe";
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return "";
    }
}

fn latest_install_kind() -> Result<InstallKind, String> {
    let suffix = asset_suffix();
    if suffix.is_empty() {
        return Err("Piattaforma non supportata".into());
    }

    let assets = fetch_release_assets(CONFIG.github_latest_url)?;
    let asset = assets
        .into_iter()
        .find(|a| a.name.ends_with(suffix))
        .ok_or_else(|| format!("Asset *{suffix} non trovato nella release"))?;

    #[cfg(target_os = "windows")]
    {
        return Ok(InstallKind::WindowsSetup {
            url: asset.browser_download_url,
            name: asset.name,
            digest: asset.digest,
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(InstallKind::AppTarGz {
            url: asset.browser_download_url,
            name: asset.name,
            digest: asset.digest,
        })
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_app_manager(app: AppHandle) -> Result<AppManagerStatus, String> {
    Ok(check_update_status(&CONFIG, &app, RESOLVE_OPTS))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn install_app_manager(app: AppHandle) -> Result<String, String> {
    let status = check_app_manager(app.clone())?;
    if status.installed && !status.update_available {
        return Ok(resolve_path_or_error(
            &CONFIG,
            &app,
            RESOLVE_OPTS,
            "AndroidAdwareCleaner già aggiornato",
        )?
        .to_string_lossy()
        .into_owned());
    }

    let kind = latest_install_kind()?;
    let app_handle = app.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        install_from_kind_with_progress(
            &CONFIG,
            &app_handle,
            kind,
            "Installazione completata ma eseguibile non trovato. Riavvia AndroidAdwareCleaner.",
            APP_ID,
        )
    })
    .await
    .map_err(|e| format!("Installazione interrotta: {e}"))??;

    Ok(installed.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn launch_app_manager(app: AppHandle) -> Result<(), String> {
    launch_installed(&CONFIG, &app, RESOLVE_OPTS)
}
