use crate::partner_app::{
    check_update_status, fetch_release_assets, install_from_kind_with_progress, launch_installed,
    resolve_path_or_error, InstallKind, PartnerAppConfig, PartnerUpdateStatus, ReleaseAsset,
    ResolveOptions,
};
use tauri::AppHandle;

const APP_ID: &str = "autobackup";

const CONFIG: PartnerAppConfig = PartnerAppConfig {
    install_folder: "AutoBackup",
    github_latest_url: "https://api.github.com/repos/sebastianoboem/AutoBackup/releases/latest",
    app_bundle_name: "AutoBackup.app",
    dev_env_var: "AUTOBACKUP_DEV",
    dev_default_path: Some("/Users/ilpano/Projects/AutoBackup"),
    legacy_mac_binary: Some("AutoBackupMAC"),
    legacy_win_binary: Some("AutoBackupPC-ANDROID.exe"),
    windows_exe_basenames: &["AutoBackup", "autobackup", "AutoBackupPC-ANDROID"],
    not_installed_error: "AutoBackup non è installato",
};

const RESOLVE_OPTS: ResolveOptions = ResolveOptions {
    allow_files: true,
};

pub type AutoBackupStatus = PartnerUpdateStatus;

fn pick_install_asset(assets: Vec<ReleaseAsset>) -> Result<InstallKind, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let preferred_suffixes = ["aarch64.app.tar.gz", "app.tar.gz"];
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let preferred_suffixes = ["x64.app.tar.gz", "app.tar.gz"];
    #[cfg(target_os = "windows")]
    let preferred_suffixes = ["x64-setup.exe"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let preferred_suffixes: [&str; 0] = [];

    for suffix in preferred_suffixes {
        if let Some(asset) = assets.iter().find(|a| a.name.ends_with(suffix)) {
            #[cfg(target_os = "windows")]
            {
                return Ok(InstallKind::WindowsSetup {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                });
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Ok(InstallKind::AppTarGz {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                });
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(asset) = assets.iter().find(|a| a.name.ends_with("-setup.exe")) {
            return Ok(InstallKind::WindowsSetup {
                url: asset.browser_download_url.clone(),
                name: asset.name.clone(),
            });
        }
        if let Some(name) = CONFIG.legacy_win_binary {
            if let Some(asset) = assets.iter().find(|a| a.name == name) {
                return Ok(InstallKind::LegacyBinary {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                });
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(name) = CONFIG.legacy_mac_binary {
            if let Some(asset) = assets.iter().find(|a| a.name == name) {
                return Ok(InstallKind::LegacyBinary {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                });
            }
        }
    }

    Err(
        "Nessun installer Windows trovato nella release AutoBackup su GitHub. \
         Pubblica prima AutoBackup_*_x64-setup.exe nella release."
            .into(),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_autobackup(app: AppHandle) -> Result<AutoBackupStatus, String> {
    Ok(check_update_status(&CONFIG, &app, RESOLVE_OPTS))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn install_autobackup(app: AppHandle) -> Result<String, String> {
    let status = check_autobackup(app.clone())?;
    if status.installed && !status.update_available {
        return Ok(
            resolve_path_or_error(&CONFIG, &app, RESOLVE_OPTS, "AutoBackup già aggiornato")?
                .to_string_lossy()
                .into_owned(),
        );
    }

    let assets = fetch_release_assets(CONFIG.github_latest_url)?;
    let kind = pick_install_asset(assets)?;
    let app_handle = app.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        install_from_kind_with_progress(
            &CONFIG,
            &app_handle,
            kind,
            "Installazione completata ma eseguibile non trovato. Riavvia AutoBackup.",
            APP_ID,
        )
    })
    .await
    .map_err(|e| format!("Installazione interrotta: {e}"))??;

    Ok(installed.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn launch_autobackup(app: AppHandle) -> Result<(), String> {
    launch_installed(&CONFIG, &app, RESOLVE_OPTS)
}
