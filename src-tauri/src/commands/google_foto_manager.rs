use crate::partner_app::{
    check_update_status, fetch_release_assets, install_from_kind_with_progress, launch_installed,
    resolve_path_or_error, InstallKind, PartnerAppConfig, PartnerUpdateStatus, ReleaseAsset,
    ResolveOptions,
};
use tauri::AppHandle;

const APP_ID: &str = "google_foto_manager";

const CONFIG: PartnerAppConfig = PartnerAppConfig {
    install_folder: "GoogleFotoManager",
    github_latest_url:
        "https://api.github.com/repos/sebastianoboem/GoogleFotoManager/releases/latest",
    app_bundle_name: "Google Foto Manager.app",
    dev_env_var: "GOOGLE_FOTO_MANAGER_DEV",
    dev_default_path: Some("/Users/ilpano/Projects/GoogleFotoManager"),
    legacy_mac_binary: None,
    legacy_win_binary: None,
    windows_exe_basenames: &[
        "Google Foto Manager",
        "Google.Foto.Manager",
        "google-foto-manager",
    ],
    not_installed_error: "GoogleFotoManager non è installato",
    dev_electron: true,
};

const RESOLVE_OPTS: ResolveOptions = ResolveOptions {
    allow_files: false,
};

pub type GoogleFotoManagerStatus = PartnerUpdateStatus;

fn pick_install_asset(assets: Vec<ReleaseAsset>) -> Result<InstallKind, String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let preferred_suffixes = ["-arm64-AppleSilicon.dmg"];
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let preferred_suffixes = ["-x64-Intel.dmg"];
    #[cfg(target_os = "windows")]
    let preferred_suffixes = [".exe"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let preferred_suffixes: [&str; 0] = [];

    for suffix in preferred_suffixes {
        if let Some(asset) = assets.iter().find(|a| a.name.ends_with(suffix)) {
            #[cfg(target_os = "windows")]
            {
                return Ok(InstallKind::WindowsSetup {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                    digest: asset.digest.clone(),
                });
            }
            #[cfg(target_os = "macos")]
            {
                return Ok(InstallKind::Dmg {
                    url: asset.browser_download_url.clone(),
                    name: asset.name.clone(),
                    digest: asset.digest.clone(),
                });
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(asset) = assets.iter().find(|a| a.name.ends_with(".dmg")) {
            return Ok(InstallKind::Dmg {
                url: asset.browser_download_url.clone(),
                name: asset.name.clone(),
                digest: asset.digest.clone(),
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.ends_with(".exe") && !a.name.ends_with(".blockmap"))
        {
            return Ok(InstallKind::WindowsSetup {
                url: asset.browser_download_url.clone(),
                name: asset.name.clone(),
                digest: asset.digest.clone(),
            });
        }
    }

    Err(
        "Nessun installer trovato nella release GoogleFotoManager su GitHub. \
         Pubblica prima gli artefatti per la tua piattaforma."
            .into(),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_google_foto_manager(app: AppHandle) -> Result<GoogleFotoManagerStatus, String> {
    Ok(check_update_status(&CONFIG, &app, RESOLVE_OPTS))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn install_google_foto_manager(app: AppHandle) -> Result<String, String> {
    let status = check_google_foto_manager(app.clone())?;
    if status.installed && !status.update_available {
        return Ok(resolve_path_or_error(
            &CONFIG,
            &app,
            RESOLVE_OPTS,
            "GoogleFotoManager già aggiornato",
        )?
        .to_string_lossy()
        .into_owned());
    }

    let assets = fetch_release_assets(CONFIG.github_latest_url)?;
    let kind = pick_install_asset(assets)?;
    let app_handle = app.clone();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        install_from_kind_with_progress(
            &CONFIG,
            &app_handle,
            kind,
            "Installazione completata ma eseguibile non trovato. Riavvia GoogleFotoManager.",
            APP_ID,
        )
    })
    .await
    .map_err(|e| format!("Installazione interrotta: {e}"))??;

    Ok(installed.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn launch_google_foto_manager(app: AppHandle) -> Result<(), String> {
    launch_installed(&CONFIG, &app, RESOLVE_OPTS)
}
