use super::state::AppState;
use adb_bridge::{AdbBridge, AdbDevice};
use package_scanner::ScannedPackage;
use reputation_db::{row_is_suspicious, PackageReputation};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use system_setup::{load_guides, resolve_guide, DeviceGuides, GuideStep};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

static STARTUP_SYNC_DONE: AtomicBool = AtomicBool::new(false);
static CLOSE_ALLOWED: AtomicBool = AtomicBool::new(false);

pub fn mark_startup_sync_done() {
    STARTUP_SYNC_DONE.store(true, Ordering::SeqCst);
}

pub fn close_allowed() -> bool {
    CLOSE_ALLOWED.load(Ordering::SeqCst)
}

#[tauri::command(rename = "adware_allow_app_close")]
pub fn allow_app_close(app: AppHandle) {
    CLOSE_ALLOWED.store(true, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
    super::dev_log::command("allow_app_close", None);
}

#[tauri::command(rename = "adware_dev_log_enabled")]
pub fn dev_log_enabled() -> bool {
    super::dev_log::command("dev_log_enabled", None);
    super::dev_log::ENABLED
}
use uninstall_engine::{UninstallEngine, UninstallResult};

pub const SUSPICIOUS_THRESHOLD: u64 = reputation_db::SUSPICIOUS_UNINSTALL_THRESHOLD;

#[derive(Debug, Clone, Serialize)]
pub struct PackageRow {
    pub package_name: String,
    pub label: Option<String>,
    pub author: Option<String>,
    pub icon_url: Option<String>,
    pub is_system: bool,
    pub installer: Option<String>,
    pub is_device_admin: bool,
    pub sideloaded: bool,
    pub installed_recently: bool,
    pub reads_screen: bool,
    pub draws_over: bool,
    pub no_launcher_icon: bool,
    pub uninstall_count: u64,
    pub report_count: u64,
    pub marked_system: bool,
    pub marked_trusted: bool,
    pub marked_suspicious: bool,
    pub is_suspicious: bool,
    pub is_reported: bool,
    pub my_vote: Option<String>,
    pub online_system: u64,
    pub online_trusted: u64,
    pub online_suspicious: u64,
    pub is_critical: bool,
}

#[derive(Debug, Serialize)]
pub struct AdbStatus {
    pub adb_path: Option<String>,
    pub devices: Vec<AdbDevice>,
    pub has_authorized_device: bool,
}

#[derive(Debug, Serialize)]
pub struct ConnectionGuide {
    pub brand: Option<String>,
    pub model: Option<String>,
    pub resolved_brand: Option<String>,
    pub device_unusable: bool,
    pub steps: Vec<GuideStep>,
    pub tips: Vec<GuideStep>,
    pub safe_mode_steps: Option<String>,
}

#[tauri::command(rename = "adware_check_setup", rename_all = "snake_case")]
pub fn check_setup() -> system_setup::SetupStatus {
    let status = system_setup::check_setup();
    super::dev_log::command("check_setup", None);
    status
}

#[tauri::command(rename = "adware_run_setup", rename_all = "snake_case")]
pub fn run_setup(state: State<'_, AppState>) -> Result<system_setup::SetupStatus, String> {
    super::dev_log::logged("run_setup", || {
        let install_dir = state.platform_tools_dir.clone();
        let status = system_setup::run_setup(&install_dir)?;
        std::env::set_var(
            "ANDROID_ADWARE_PLATFORM_TOOLS",
            install_dir.to_string_lossy().as_ref(),
        );
        Ok(status)
    })
}

#[tauri::command(rename = "adware_get_device_guides", rename_all = "snake_case")]
pub fn get_device_guides() -> DeviceGuides {
    let guides = load_guides();
    super::dev_log::command("get_device_guides", None);
    guides
}

#[tauri::command(rename = "adware_get_connection_guide", rename_all = "snake_case")]
pub fn get_connection_guide(
    brand: Option<String>,
    model: Option<String>,
    device_unusable: bool,
) -> ConnectionGuide {
    let guides = load_guides();
    let brand_ref = brand.as_deref().unwrap_or("").trim();
    let model_ref = model.as_deref().unwrap_or("").trim();

    let matched = resolve_guide(&guides, brand_ref, model_ref);
    let resolved_brand = matched.map(|g| g.brand.clone());

    let steps: Vec<GuideStep> = if let Some(g) = matched {
        g.steps.clone()
    } else {
        guides.generic.clone()
    };

    let mut tips: Vec<GuideStep> = if let Some(g) = matched {
        g.tips.clone()
    } else {
        guides.generic_tips.clone()
    };

    if device_unusable {
        let has_safe_tip = tips.iter().any(|s| s.phase == "safe_mode");
        if !has_safe_tip {
            let safe_body = matched
                .map(|g| g.safe_mode_steps.clone())
                .unwrap_or_else(|| {
                    "Tieni premuto «Spegni» nel menu power finché appare Safe mode, poi selezionalo."
                        .into()
                });
            tips.push(GuideStep {
                title: "Safe Mode (se il telefono è inutilizzabile)".into(),
                body: safe_body,
                phase: "safe_mode".into(),
            });
        }
    } else {
        tips.retain(|s| s.phase != "safe_mode");
    }

    let safe_mode_steps = matched.map(|g| g.safe_mode_steps.clone());

    super::dev_log::command("get_connection_guide", None);
    ConnectionGuide {
        brand,
        model,
        resolved_brand,
        device_unusable,
        steps,
        tips,
        safe_mode_steps,
    }
}

#[tauri::command(rename = "adware_set_custom_adb_path", rename_all = "snake_case")]
pub fn set_custom_adb_path(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<system_setup::SetupStatus, String> {
    super::dev_log::logged("set_custom_adb_path", || {
        {
            let mut config = state.config.lock().map_err(|e| e.to_string())?;
            config.custom_adb_path = path.clone();
        }
        state.save_config()?;
        if let Some(ref p) = path {
            std::env::set_var("ANDROID_ADWARE_CUSTOM_ADB", p);
        } else {
            std::env::remove_var("ANDROID_ADWARE_CUSTOM_ADB");
        }
        Ok(system_setup::check_setup())
    })
}

#[tauri::command(rename = "adware_get_adb_status", rename_all = "snake_case")]
pub fn adware_get_adb_status() -> Result<AdbStatus, String> {
    let adb_path = adb_bridge::resolve_adb_path().map(|p| p.to_string_lossy().into_owned());
    let devices = match AdbBridge::new() {
        Ok(b) => b.list_devices().unwrap_or_default(),
        Err(_) => vec![],
    };
    let has_authorized_device = devices
        .iter()
        .any(|d| d.state == adb_bridge::DeviceState::Device);
    let status = Ok(AdbStatus {
        adb_path,
        devices,
        has_authorized_device,
    });
    super::dev_log::log_result("get_adb_status", &status);
    status
}

fn sync_status(state: &AppState) -> Result<cloud_sync::SyncStatus, String> {
    let provider_id = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config
            .sync_provider
            .clone()
            .unwrap_or_else(|| "local".into())
    };
    let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
    let mut status = sync.status_for(&provider_id);
    status.startup_pending = !STARTUP_SYNC_DONE.load(Ordering::SeqCst);
    Ok(status)
}

async fn reconcile_off_main(app: AppHandle) -> Result<cloud_sync::SyncStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        reconcile_state(state.inner())
    })
    .await
    .map_err(|e| format!("sync interrotta: {e}"))?
}

#[tauri::command(rename = "adware_sync_pull", rename_all = "snake_case")]
pub async fn sync_pull(app: AppHandle) -> Result<cloud_sync::SyncStatus, String> {
    let result = reconcile_off_main(app).await;
    super::dev_log::log_result("sync_pull", &result);
    result
}

pub fn reconcile_quiet(state: &AppState) {
    let _ = reconcile_state_inner(state);
}

#[derive(Debug, Serialize)]
pub struct StartupDbPhase {
    pub remote: bool,
    pub startup_pending: bool,
}

#[tauri::command(rename = "adware_startup_db_phase", rename_all = "snake_case")]
pub fn startup_db_phase(state: State<'_, AppState>) -> Result<StartupDbPhase, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let provider = config.sync_provider.as_deref().unwrap_or("local");
    let remote = match provider {
        "local" => false,
        "supabase" => {
            config
                .supabase_url
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
                && config
                    .supabase_key
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        }
        _ => config
            .sync_folder
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty()),
    };
    Ok(StartupDbPhase {
        remote,
        startup_pending: !STARTUP_SYNC_DONE.load(Ordering::SeqCst),
    })
}

fn reconcile_state(state: &AppState) -> Result<cloud_sync::SyncStatus, String> {
    reconcile_state_inner(state)
}

fn reconcile_state_inner(state: &AppState) -> Result<cloud_sync::SyncStatus, String> {
    let mut sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    sync.reconcile(&db).map_err(|e| e.to_string())?;
    drop(db);
    drop(sync);
    sync_status(state)
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScanProgressEvent {
    Started {
        total: usize,
        device_serial: String,
        device_model: Option<String>,
        device_brand: Option<String>,
    },
    Package {
        row: PackageRow,
    },
    Finished,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UninstallProgressEvent {
    Started {
        total: usize,
    },
    Item {
        current: usize,
        total: usize,
        package_name: String,
        status: String,
        message: String,
    },
    Finished,
}

struct ScanMeta {
    model: Option<String>,
    brand: Option<String>,
    critical: HashSet<String>,
    reputations: Result<HashMap<String, PackageReputation>, String>,
}

fn scan_packages_streaming(
    user_only: bool,
    serial: Option<String>,
    reputations: mpsc::Receiver<Result<HashMap<String, PackageReputation>, String>>,
    on_progress: Channel<ScanProgressEvent>,
) -> Result<(), String> {
    let bridge = Arc::new(match serial {
        Some(s) => AdbBridge::with_serial(s).map_err(|e| e.to_string())?,
        None => AdbBridge::new().map_err(|e| e.to_string())?,
    });
    let device_serial = bridge.get_serial().map_err(|e| e.to_string())?;

    let model_bridge = Arc::clone(&bridge);
    let mut model_handle = Some(thread::spawn(move || {
        model_bridge.get_device_property("ro.product.model").ok()
    }));
    let brand_bridge = Arc::clone(&bridge);
    let mut brand_handle = Some(thread::spawn(move || {
        brand_bridge.get_device_property("ro.product.brand").ok()
    }));
    let crit_bridge = Arc::clone(&bridge);
    let mut crit_handle = Some(thread::spawn(move || {
        uninstall_engine::dynamic_critical(&crit_bridge)
    }));
    let mut reputations = Some(reputations);

    let scanner = package_scanner::PackageScanner::new(Arc::clone(&bridge));
    let mut meta: Option<ScanMeta> = None;
    let mut rep_error: Option<String> = None;
    scanner
        .scan_with_progress(user_only, |event| {
            let meta = meta.get_or_insert_with(|| ScanMeta {
                model: model_handle
                    .take()
                    .and_then(|handle| handle.join().ok().flatten()),
                brand: brand_handle
                    .take()
                    .and_then(|handle| handle.join().ok().flatten()),
                critical: crit_handle
                    .take()
                    .and_then(|handle| handle.join().ok())
                    .unwrap_or_default(),
                reputations: reputations
                    .take()
                    .and_then(|rx| rx.recv().ok())
                    .unwrap_or_else(|| Err("reputazioni non disponibili".into())),
            });
            if let Err(err) = &meta.reputations {
                if rep_error.is_none() {
                    rep_error = Some(err.clone());
                }
                return;
            }
            let reputations = meta.reputations.as_ref().unwrap();
            match event {
                package_scanner::ScanProgress::Started { total } => {
                    let _ = on_progress.send(ScanProgressEvent::Started {
                        total,
                        device_serial: device_serial.clone(),
                        device_model: meta.model.clone(),
                        device_brand: meta.brand.clone(),
                    });
                }
                package_scanner::ScanProgress::Package(pkg) => {
                    let row = package_to_row(pkg, reputations, &meta.critical);
                    let _ = on_progress.send(ScanProgressEvent::Package { row });
                }
            }
        })
        .map_err(|e| e.to_string())?;

    if meta.is_none() {
        let _ = model_handle.take().map(|handle| handle.join());
        let _ = brand_handle.take().map(|handle| handle.join());
        let _ = crit_handle.take().map(|handle| handle.join());
        let _ = reputations.take().map(|rx| rx.recv());
    }
    if let Some(err) = rep_error {
        return Err(err);
    }
    let _ = on_progress.send(ScanProgressEvent::Finished);
    Ok(())
}

fn load_reputations(
    state: &AppState,
    viewer: Option<String>,
) -> Result<std::collections::HashMap<String, PackageReputation>, String> {
    {
        let mut sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
        if sync.is_supabase() {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let _ = sync.pull_stats(&db);
        }
    }
    let pending = state
        .pending_push
        .lock()
        .map(|set| set.clone())
        .unwrap_or_default();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db
        .all_reputations_pending(viewer.as_deref(), &pending)
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.package_name.clone(), r))
        .collect())
}

#[tauri::command(rename = "adware_scan_packages", rename_all = "snake_case")]
pub async fn scan_packages(
    app: AppHandle,
    _state: State<'_, AppState>,
    user_only: bool,
    serial: Option<String>,
    on_progress: Channel<ScanProgressEvent>,
) -> Result<(), String> {
    let result = async {
        if let Some(serial) = serial.as_ref().filter(|s| !s.is_empty()) {
            remember_viewer(_state.inner(), serial);
        }
        let viewer = if serial.as_ref().is_some_and(|s| !s.is_empty()) {
            serial.clone()
        } else {
            AdbBridge::new()
                .ok()
                .and_then(|bridge| bridge.get_serial().ok())
        };
        let (rep_tx, rep_rx) = mpsc::channel();
        let app_for_rep = app.clone();
        let rep_task = tauri::async_runtime::spawn_blocking(move || {
            let state = app_for_rep.state::<AppState>();
            let _ = rep_tx.send(load_reputations(state.inner(), viewer));
        });
        let scan_result = tauri::async_runtime::spawn_blocking(move || {
            scan_packages_streaming(user_only, serial, rep_rx, on_progress)
        })
        .await
        .map_err(|e| format!("scansione interrotta: {e}"))?;
        let _ = rep_task.await;
        scan_result
    }
    .await;
    super::dev_log::log_result("scan_packages", &result);
    result
}

fn package_to_row(
    p: ScannedPackage,
    reputations: &std::collections::HashMap<String, PackageReputation>,
    critical: &std::collections::HashSet<String>,
) -> PackageRow {
    let rep = reputations.get(&p.package_name);
    let uninstall_count = rep.map(|r| r.uninstall_count).unwrap_or(0);
    let report_count = rep.map(|r| r.report_count).unwrap_or(0);
    let marked_system = rep.map(|r| r.marked_system).unwrap_or(false);
    let marked_trusted = rep.map(|r| r.marked_trusted).unwrap_or(false);
    let marked_suspicious = rep.map(|r| r.marked_suspicious).unwrap_or(false);
    let is_reported = marked_suspicious;
    let is_suspicious = rep
        .map(|r| row_is_suspicious(r, p.is_system))
        .unwrap_or_else(|| !p.is_system && uninstall_count > SUSPICIOUS_THRESHOLD);

    let is_critical = uninstall_engine::is_critical(&p.package_name, critical);
    PackageRow {
        package_name: p.package_name,
        label: p.label,
        author: p.author,
        icon_url: p.icon_url,
        is_system: p.is_system,
        installer: p.installer,
        is_device_admin: p.is_device_admin,
        sideloaded: p.sideloaded,
        installed_recently: p.installed_recently,
        reads_screen: p.reads_screen,
        draws_over: p.draws_over,
        no_launcher_icon: p.no_launcher_icon,
        uninstall_count,
        report_count,
        marked_system,
        marked_trusted,
        marked_suspicious,
        is_suspicious,
        is_reported,
        my_vote: rep.and_then(|r| r.my_vote.clone()),
        online_system: rep.map(|r| r.online_system).unwrap_or(0),
        online_trusted: rep.map(|r| r.online_trusted).unwrap_or(0),
        online_suspicious: rep.map(|r| r.online_suspicious).unwrap_or(0),
        is_critical,
    }
}

#[derive(Debug, Deserialize)]
pub struct UninstallRequest {
    pub packages: Vec<String>,
    pub dry_run: bool,
    pub allow_critical: bool,
    pub serial: Option<String>,
}

#[tauri::command(rename = "adware_bulk_uninstall", rename_all = "snake_case")]
pub async fn bulk_uninstall(
    state: State<'_, AppState>,
    req: UninstallRequest,
    on_progress: Channel<UninstallProgressEvent>,
) -> Result<(), String> {
    let result = async {
        let packages = req.packages.clone();
        let dry_run = req.dry_run;
        let allow_critical = req.allow_critical;
        let serial = req.serial.clone();
        let total = packages.len();
        let on_progress_worker = on_progress.clone();

        let results = tauri::async_runtime::spawn_blocking(move || {
            let _ = on_progress_worker.send(UninstallProgressEvent::Started { total });

            let bridge = match serial {
                Some(s) => AdbBridge::with_serial(s).map_err(|e| e.to_string())?,
                None => AdbBridge::new().map_err(|e| e.to_string())?,
            };
            let device_serial = bridge.get_serial().map_err(|e| e.to_string())?;
            let engine = UninstallEngine::new(bridge);
            let results = engine
                .bulk_uninstall_with_progress(
                    &packages,
                    dry_run,
                    allow_critical,
                    |current, total, result| {
                        let status = match result.status {
                            uninstall_engine::UninstallStatus::Success => "success",
                            uninstall_engine::UninstallStatus::Failed => "failed",
                            uninstall_engine::UninstallStatus::Skipped => "skipped",
                        };
                        let _ = on_progress_worker.send(UninstallProgressEvent::Item {
                            current,
                            total,
                            package_name: result.package_name.clone(),
                            status: status.into(),
                            message: result.message.clone(),
                        });
                    },
                )
                .map_err(|e| e.to_string())?;

            Ok::<(Vec<UninstallResult>, String), String>((results, device_serial))
        })
        .await
        .map_err(|e| format!("disinstallazione interrotta: {e}"))??;

        let (results, device_serial) = results;

        if !dry_run {
            let mut events = Vec::new();
            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                for r in &results {
                    if r.status == uninstall_engine::UninstallStatus::Success {
                        if let Ok(event) = db.record_uninstall(
                            &r.package_name,
                            &device_serial,
                            true,
                            Some(&r.message),
                        ) {
                            events.push(event);
                        }
                    }
                }
            }
            remember_viewer(state.inner(), &device_serial);
            for event in &events {
                publish_change(state.inner(), &event.package_name, |sync| {
                    sync.push_uninstall(event)
                });
            }
        }

        let _ = on_progress.send(UninstallProgressEvent::Finished);
        Ok(())
    }
    .await;
    super::dev_log::log_result("bulk_uninstall", &result);
    result
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub packages: Vec<String>,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    pub package_name: String,
    pub ok: bool,
    pub message: String,
}

fn restore_message_ok(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("installed") && !lower.contains("failure") && !lower.contains("exception")
}

#[tauri::command(rename = "adware_restore_packages", rename_all = "snake_case")]
pub async fn restore_packages(req: RestoreRequest) -> Result<Vec<RestoreResult>, String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(req.serial)?;
            let mut results = Vec::with_capacity(req.packages.len());
            for package in req.packages {
                let (ok, message) = match bridge.install_existing(&package) {
                    Ok(message) => (restore_message_ok(&message), message),
                    Err(err) => (false, err.to_string()),
                };
                results.push(RestoreResult {
                    package_name: package,
                    ok,
                    message,
                });
            }
            Ok(results)
        })
        .await
        .map_err(|e| format!("ripristino interrotto: {e}"))?
    }
    .await;
    super::dev_log::log_result("restore_packages", &result);
    result
}

#[tauri::command(rename = "adware_check_browsers", rename_all = "snake_case")]
pub async fn check_browsers(
    serial: Option<String>,
) -> Result<super::browsers::BrowserCheck, String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(serial)?;
            super::browsers::check(&bridge)
        })
        .await
        .map_err(|e| format!("controllo browser interrotto: {e}"))?
    }
    .await;
    super::dev_log::log_result("check_browsers", &result);
    result
}

#[tauri::command(rename = "adware_clear_browsing_data", rename_all = "snake_case")]
pub async fn clear_browsing_data(package: String, serial: Option<String>) -> Result<(), String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(serial)?;
            super::clear_browsing::clear(&bridge, &package)
        })
        .await
        .map_err(|e| format!("pulizia browser interrotta: {e}"))?
    }
    .await;
    super::dev_log::log_result("clear_browsing_data", &result);
    result
}

#[tauri::command(rename = "adware_open_notification_settings", rename_all = "snake_case")]
pub async fn open_notification_settings(
    package: String,
    serial: Option<String>,
) -> Result<(), String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(serial)?;
            bridge
                .open_notification_settings(&package)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("apertura notifiche interrotta: {e}"))?
    }
    .await;
    super::dev_log::log_result("open_notification_settings", &result);
    result
}

#[tauri::command(rename = "adware_open_default_apps", rename_all = "snake_case")]
pub async fn open_default_apps(serial: Option<String>) -> Result<(), String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(serial)?;
            bridge
                .open_default_apps()
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("apertura app predefinite interrotta: {e}"))?
    }
    .await;
    super::dev_log::log_result("open_default_apps", &result);
    result
}

#[tauri::command(rename = "adware_set_browser_notifications", rename_all = "snake_case")]
pub async fn set_browser_notifications(
    package: String,
    allowed: bool,
    serial: Option<String>,
) -> Result<(), String> {
    let result = async {
        tauri::async_runtime::spawn_blocking(move || {
            let bridge = open_bridge(serial)?;
            bridge
                .set_notifications_allowed(&package, allowed)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("notifiche browser interrotte: {e}"))?
    }
    .await;
    super::dev_log::log_result("set_browser_notifications", &result);
    result
}

#[derive(Debug, Deserialize)]
pub struct SetPackageMarksRequest {
    pub package_name: String,
    pub device_serial: String,
    pub flag: String,
}

#[tauri::command(rename = "adware_set_package_marks", rename_all = "snake_case")]
pub fn set_package_marks(
    state: State<'_, AppState>,
    req: SetPackageMarksRequest,
) -> Result<PackageReputation, String> {
    super::dev_log::logged("set_package_marks", || {
        let vote = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            db.set_vote(&req.package_name, &req.device_serial, &req.flag)
                .map_err(|e| e.to_string())?
        };
        remember_viewer(state.inner(), &req.device_serial);
        publish_change(state.inner(), &vote.package_name, |sync| {
            sync.push_vote(&vote)
        });
        let pending = state
            .pending_push
            .lock()
            .map(|set| set.clone())
            .unwrap_or_default();
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_reputation_pending(&req.package_name, Some(&req.device_serial), &pending)
            .map_err(|e| e.to_string())
    })
}

fn remember_viewer(state: &AppState, serial: &str) {
    if serial.is_empty() {
        return;
    }
    if let Ok(mut slot) = state.viewer_serial.lock() {
        *slot = Some(serial.to_string());
    }
}

fn publish_change(
    state: &AppState,
    package: &str,
    push: impl FnOnce(&mut cloud_sync::CloudSync) -> Result<(), cloud_sync::SyncError>,
) {
    let supabase = state
        .cloud_sync
        .lock()
        .map(|sync| sync.is_supabase())
        .unwrap_or(false);
    if !supabase {
        return;
    }
    if let Ok(mut pending) = state.pending_push.lock() {
        pending.insert(package.to_string());
    }
    let pushed = state
        .cloud_sync
        .lock()
        .map(|mut sync| push(&mut sync).is_ok())
        .unwrap_or(false);
    if !pushed {
        return;
    }
    if let (Ok(mut sync), Ok(db)) = (state.cloud_sync.lock(), state.db.lock()) {
        let _ = sync.pull_package(&db, package);
    }
    if let Ok(mut pending) = state.pending_push.lock() {
        pending.remove(package);
    }
}

#[tauri::command(rename = "adware_get_sync_status", rename_all = "snake_case")]
pub async fn get_sync_status(app: AppHandle) -> Result<cloud_sync::SyncStatus, String> {
    let result = async move {
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<AppState>();
            sync_status(state.inner())
        })
        .await
        .map_err(|e| format!("stato sync interrotto: {e}"))?
    }
    .await;
    super::dev_log::log_result("get_sync_status", &result);
    result
}

#[tauri::command(rename = "adware_get_sync_settings", rename_all = "snake_case")]
pub fn get_sync_settings(
    state: State<'_, AppState>,
) -> Result<cloud_sync::SyncSettingsView, String> {
    super::dev_log::logged("get_sync_settings", || {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
        let provider_id = config
            .sync_provider
            .clone()
            .unwrap_or_else(|| "local".into());
        Ok(cloud_sync::SyncSettingsView {
            provider_id,
            sync_folder: config.sync_folder.clone().or_else(|| {
                sync.sync_path()
                    .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
            }),
            subfolder: cloud_sync::SYNC_SUBFOLDER.into(),
            providers: cloud_sync::list_providers(),
            supabase_url: config.supabase_url.clone(),
            supabase_key: config.supabase_key.clone(),
        })
    })
}

fn apply_sync_folder(
    state: &AppState,
    provider_id: &str,
    folder: Option<String>,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.sync_provider = Some(provider_id.into());
        config.sync_folder = folder.clone();
    }
    state.save_config()?;
    reload_backend(state)
}

fn reload_backend(state: &AppState) -> Result<(), String> {
    let (provider, folder, url, key) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (
            config.sync_provider.clone(),
            config.sync_folder.clone(),
            config.supabase_url.clone(),
            config.supabase_key.clone(),
        )
    };
    let session_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("supabase-session.json");
    let backend = cloud_sync::backend_for(
        provider.as_deref(),
        folder.map(std::path::PathBuf::from),
        session_path,
        url.as_deref(),
        key.as_deref(),
    );
    let mut sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
    sync.set_backend(backend);
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pushed = sync.reconcile(&db).is_ok();
    drop(db);
    drop(sync);
    if pushed {
        if let Ok(mut pending) = state.pending_push.lock() {
            pending.clear();
        }
    }
    state
        .realtime_gen
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command(rename = "adware_set_sync_provider", rename_all = "snake_case")]
pub fn set_sync_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<cloud_sync::SyncStatus, String> {
    super::dev_log::logged("set_sync_provider", || {
        if provider_id == "local" {
            apply_sync_folder(&state, "local", None)?;
            let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
            return Ok(sync.status());
        }
        if provider_id == "supabase" {
            let ready = {
                let config = state.config.lock().map_err(|e| e.to_string())?;
                config
                    .supabase_url
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
                    && config
                        .supabase_key
                        .as_ref()
                        .is_some_and(|s| !s.trim().is_empty())
            };
            if !ready {
                return Err("inserisci URL e chiave publishable".into());
            }
            apply_sync_folder(&state, "supabase", None)?;
            let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
            return Ok(sync.status());
        }
        if provider_id == "custom" {
            return Err("usa set_sync_folder per una cartella personalizzata".into());
        }
        let folder = cloud_sync::resolve_sync_folder(&provider_id, None).ok_or_else(|| {
            format!(
                "cartella {} non trovata — installa il client sul PC o usa «Cartella personalizzata»",
                provider_id
            )
        })?;
        cloud_sync::ensure_sync_folder(&folder).map_err(|e| e.to_string())?;
        let folder_str = folder.to_string_lossy().into_owned();
        apply_sync_folder(&state, &provider_id, Some(folder_str))?;
        let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
        Ok(sync.status())
    })
}

#[tauri::command(rename = "adware_set_supabase_config", rename_all = "snake_case")]
pub fn set_supabase_config(
    state: State<'_, AppState>,
    url: String,
    key: String,
) -> Result<cloud_sync::SyncStatus, String> {
    super::dev_log::logged("set_supabase_config", || {
        let url = url.trim().trim_end_matches('/').to_string();
        let key = key.trim().to_string();
        if url.is_empty() || key.is_empty() {
            return Err("servono URL del progetto e chiave publishable".into());
        }
        {
            let mut config = state.config.lock().map_err(|e| e.to_string())?;
            config.sync_provider = Some("supabase".into());
            config.sync_folder = None;
            config.supabase_url = Some(url);
            config.supabase_key = Some(key);
        }
        state.save_config()?;
        reload_backend(&state)?;
        let sync = state.cloud_sync.lock().map_err(|e| e.to_string())?;
        Ok(sync.status())
    })
}

#[tauri::command(rename = "adware_set_sync_folder", rename_all = "snake_case")]
pub fn set_sync_folder(state: State<'_, AppState>, folder: Option<String>) -> Result<(), String> {
    super::dev_log::logged("set_sync_folder", || {
        if let Some(ref path) = folder {
            cloud_sync::ensure_sync_folder(std::path::Path::new(path))
                .map_err(|e| e.to_string())?;
        }
        apply_sync_folder(&state, "custom", folder)
    })
}

#[tauri::command(rename = "adware_sync_now", rename_all = "snake_case")]
pub async fn sync_now(app: AppHandle) -> Result<cloud_sync::SyncStatus, String> {
    let result = reconcile_off_main(app).await;
    super::dev_log::log_result("sync_now", &result);
    result
}

#[derive(Debug, Deserialize)]
pub struct ExportReportRequest {
    pub device_serial: String,
    pub imei: Option<String>,
    pub removed_packages: Vec<String>,
    pub output_path: String,
    pub format: String,
}

#[tauri::command(rename = "adware_export_report", rename_all = "snake_case")]
pub fn export_report(req: ExportReportRequest) -> Result<String, String> {
    super::dev_log::logged("export_report", || {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        match req.format.as_str() {
            "csv" => export_csv(&req, &timestamp.to_string()),
            "pdf" => export_pdf(&req, &timestamp.to_string()),
            _ => Err("format must be csv or pdf".into()),
        }
    })
}

fn export_csv(req: &ExportReportRequest, timestamp: &str) -> Result<String, String> {
    let mut lines = vec![
        "field,value".to_string(),
        format!("timestamp,{timestamp}"),
        format!("device_serial,{}", req.device_serial),
        format!("imei,{}", req.imei.as_deref().unwrap_or("")),
        format!("removed_count,{}", req.removed_packages.len()),
    ];
    for (i, pkg) in req.removed_packages.iter().enumerate() {
        lines.push(format!("removed_{i},{pkg}"));
    }
    std::fs::write(&req.output_path, lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(req.output_path.clone())
}

fn export_pdf(req: &ExportReportRequest, timestamp: &str) -> Result<String, String> {
    let content = format!(
        "AndroidAdwareCleaner Work Report\n\nTimestamp: {timestamp}\nDevice serial: {}\nIMEI: {}\nPackages removed: {}\n\nRemoved:\n{}",
        req.device_serial,
        req.imei.as_deref().unwrap_or("N/A"),
        req.removed_packages.len(),
        req.removed_packages.join("\n")
    );
    let pdf = build_simple_pdf(&content);
    std::fs::write(&req.output_path, pdf).map_err(|e| e.to_string())?;
    Ok(req.output_path.clone())
}

fn build_simple_pdf(text: &str) -> Vec<u8> {
    let escaped: String = text
        .chars()
        .map(|c| match c {
            '(' | ')' | '\\' => format!("\\{c}"),
            '\n' => "\\n".to_string(),
            _ => c.to_string(),
        })
        .collect();
    let stream = format!("BT /F1 10 Tf 50 750 Td ({escaped}) Tj ET");
    let stream_len = stream.len();
    let pdf = format!(
        "%PDF-1.4\n\
        1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
        2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
        3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n\
        4 0 obj<</Length {stream_len}>>stream\n{stream}\nendstream endobj\n\
        5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
        xref\n0 6\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000266 00000 n \n0000000{} 00000 n \n\
        trailer<</Size 6/Root 1 0 R>>\nstartxref\n{}\n%%EOF",
        350 + stream_len,
        350 + stream_len
    );
    pdf.into_bytes()
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OptimizeProgressEvent {
    Started {
        total: usize,
    },
    Item {
        current: usize,
        total: usize,
        label: String,
        status: String,
        message: String,
    },
    Finished {
        freed_bytes: u64,
    },
}

#[derive(Debug, Deserialize)]
pub struct CleanStorageRequest {
    pub scan_id: u64,
    pub categories: Vec<String>,
    pub large_files: Vec<String>,
    pub serial: Option<String>,
}

fn open_bridge(serial: Option<String>) -> Result<AdbBridge, String> {
    match serial.filter(|s| !s.is_empty()) {
        Some(serial) => AdbBridge::with_serial(serial).map_err(|e| e.to_string()),
        None => AdbBridge::new().map_err(|e| e.to_string()),
    }
}

#[derive(Clone, Serialize)]
pub struct OptimizeScanProgress {
    pub label: String,
}

#[tauri::command(rename = "adware_scan_storage", rename_all = "snake_case")]
pub async fn scan_storage(
    state: State<'_, AppState>,
    serial: Option<String>,
    on_progress: Channel<OptimizeScanProgress>,
) -> Result<super::optimizer::OptimizeScan, String> {
    let result = async {
        let scan_id = {
            let mut slot = state.optimize.lock().map_err(|e| e.to_string())?;
            slot.seq += 1;
            slot.seq
        };
        let mut snapshot = tauri::async_runtime::spawn_blocking(move || {
            super::optimizer::scan(&open_bridge(serial)?, |label| {
                let _ = on_progress.send(OptimizeScanProgress {
                    label: label.to_string(),
                });
            })
        })
        .await
        .map_err(|e| format!("analisi interrotta: {e}"))??;
        snapshot.scan_id = scan_id;
        let view = super::optimizer::OptimizeScan {
            scan_id,
            categories: snapshot.categories.clone(),
            large_files: snapshot.large_files.clone(),
        };
        let mut slot = state.optimize.lock().map_err(|e| e.to_string())?;
        if slot.seq == scan_id {
            slot.current = Some(snapshot);
        }
        Ok(view)
    }
    .await;
    super::dev_log::log_result("scan_storage", &result);
    result
}

#[tauri::command(rename = "adware_clean_storage", rename_all = "snake_case")]
pub async fn clean_storage(
    state: State<'_, AppState>,
    req: CleanStorageRequest,
    on_progress: Channel<OptimizeProgressEvent>,
) -> Result<super::optimizer::CleanResult, String> {
    let result = async {
        let snapshot = {
            let slot = state.optimize.lock().map_err(|e| e.to_string())?;
            let current = slot.current.as_ref().ok_or("Nessuna scansione")?;
            if current.scan_id != req.scan_id {
                return Err("Scansione non più valida".into());
            }
            current.clone()
        };
        let ops = super::optimizer::plan(&snapshot, &req.categories, &req.large_files)?;
        let serial = req.serial;
        let on_progress_worker = on_progress.clone();
        let total = ops.len();

        tauri::async_runtime::spawn_blocking(move || {
            let _ = on_progress_worker.send(OptimizeProgressEvent::Started { total });
            let bridge = open_bridge(serial)?;
            let result = super::optimizer::execute(
                &ops,
                |command| bridge.shell(command).map_err(|e| e.to_string()),
                |current, total, label, ok, message| {
                    let _ = on_progress_worker.send(OptimizeProgressEvent::Item {
                        current,
                        total,
                        label: label.to_string(),
                        status: if ok { "success" } else { "failed" }.into(),
                        message: message.to_string(),
                    });
                },
            );
            let _ = on_progress_worker.send(OptimizeProgressEvent::Finished {
                freed_bytes: result.freed_bytes,
            });
            Ok(result)
        })
        .await
        .map_err(|e| format!("pulizia interrotta: {e}"))?
    }
    .await;
    super::dev_log::log_result("clean_storage", &result);
    result
}
