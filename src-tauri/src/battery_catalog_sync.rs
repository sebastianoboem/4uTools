use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use device_info::{
    replace_battery_catalog, BatteryCatalog, BatteryCatalogEntry, BatteryCatalogFile,
};
use regex::Regex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const SITEMAP_URL: &str = "https://andreagaleazzi.com/schede-tecniche-sitemap.xml";
const USER_AGENT: &str = "4uTools/1.3.2 (+https://github.com/sebastianoboem/4uTools; battery-catalog)";
const WORKERS: usize = 8;
const CATALOG_FILENAME: &str = "battery-catalog.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryCatalogStatus {
    pub entries: usize,
    pub updated_at: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    current: usize,
    total: usize,
    phase: String,
}

pub fn catalog_user_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("battery");
    fs::create_dir_all(&dir).map_err(|e| format!("create catalog dir: {e}"))?;
    Ok(dir.join(CATALOG_FILENAME))
}

pub fn catalog_bundled_path() -> Option<PathBuf> {
    // Dev: repo resources/
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev = manifest.join("../resources").join(CATALOG_FILENAME);
    if dev.is_file() {
        return Some(dev);
    }
    None
}

pub fn init_battery_catalog(app: &AppHandle) {
    let user = catalog_user_path(app).ok();
    if let Some(path) = user.as_ref().filter(|p| p.is_file()) {
        if device_info::load_battery_catalog_path(path).is_ok() {
            return;
        }
    }
    if let Some(path) = catalog_bundled_path() {
        let _ = device_info::load_battery_catalog_path(&path);
    }
}

pub fn catalog_status(app: &AppHandle) -> Result<BatteryCatalogStatus, String> {
    let (entries, updated_at) = device_info::battery_catalog_stats();
    let path = catalog_user_path(app)?
        .to_string_lossy()
        .into_owned();
    Ok(BatteryCatalogStatus {
        entries,
        updated_at,
        path,
    })
}

pub fn sync_battery_catalog(app: &AppHandle) -> Result<BatteryCatalogStatus, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    emit_progress(app, 0, 0, "sitemap");
    let sitemap = client
        .get(SITEMAP_URL)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| format!("sitemap: {e}"))?;

    let urls = parse_sitemap_urls(&sitemap);
    if urls.is_empty() {
        return Err("Nessuna scheda trovata nella sitemap".into());
    }

    let total = urls.len();
    emit_progress(app, 0, total, "download");

    let log_re = Regex::new(r"console\.log\('(\{.*?\})'\)").map_err(|e| e.to_string())?;
    let entries = Arc::new(Mutex::new(Vec::<BatteryCatalogEntry>::with_capacity(total)));
    let done = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    let chunk_size = (total + WORKERS - 1) / WORKERS;
    thread::scope(|scope| {
        for chunk in urls.chunks(chunk_size.max(1)) {
            let chunk = chunk.to_vec();
            let client = client.clone();
            let log_re = log_re.clone();
            let entries = Arc::clone(&entries);
            let done = Arc::clone(&done);
            let errors = Arc::clone(&errors);
            let app = app.clone();
            scope.spawn(move || {
                for url in chunk {
                    match fetch_entry(&client, &log_re, &url) {
                        Ok(Some(entry)) => {
                            if let Ok(mut g) = entries.lock() {
                                g.push(entry);
                            }
                        }
                        Ok(None) => {
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    let current = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if current == total || current % 10 == 0 {
                        emit_progress(&app, current, total, "download");
                    }
                    thread::sleep(Duration::from_millis(40));
                }
            });
        }
    });

    let mut list = entries.lock().map_err(|e| e.to_string())?.clone();
    list.sort_by(|a, b| a.slug.cmp(&b.slug));
    list.dedup_by(|a, b| a.slug == b.slug);

    let updated_at = iso_now();
    let file = BatteryCatalogFile {
        version: 1,
        updated_at: updated_at.clone(),
        source: "andreagaleazzi.com".into(),
        entries: list,
    };

    let path = catalog_user_path(app)?;
    let json = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| format!("write catalog: {e}"))?;

    // Also refresh bundled copy in dev when possible.
    if let Some(bundled) = catalog_bundled_path() {
        let _ = fs::write(bundled, &json);
    }

    replace_battery_catalog(BatteryCatalog::from_file(file));
    emit_progress(app, total, total, "done");

    Ok(BatteryCatalogStatus {
        entries: device_info::battery_catalog_stats().0,
        updated_at,
        path: path.to_string_lossy().into_owned(),
    })
}

fn fetch_entry(
    client: &reqwest::blocking::Client,
    log_re: &Regex,
    url: &str,
) -> Result<Option<BatteryCatalogEntry>, String> {
    let html = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| e.to_string())?;

    let Some(caps) = log_re.captures(&html) else {
        return Ok(None);
    };
    let obj: serde_json::Value =
        serde_json::from_str(&caps[1]).map_err(|e| e.to_string())?;

    let mah = match obj.get("batteria") {
        Some(serde_json::Value::String(s)) => s.parse::<u32>().unwrap_or(0),
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
        _ => 0,
    };
    if mah == 0 {
        return Ok(None);
    }

    let slug = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    let marca = obj
        .get("marca")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let nome = obj
        .get("nome")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Some(BatteryCatalogEntry {
        slug,
        marca,
        nome,
        batteria_mah: mah,
    }))
}

fn parse_sitemap_urls(xml: &str) -> Vec<String> {
    let re = Regex::new(r"<loc>(https://andreagaleazzi\.com/schede-tecniche/[^<]+)</loc>")
        .expect("sitemap regex");
    let mut urls: Vec<String> = re
        .captures_iter(xml)
        .map(|c| c[1].trim().to_string())
        .filter(|u| u.ends_with('/'))
        .collect();
    urls.sort();
    urls.dedup();
    urls
}

fn emit_progress(app: &AppHandle, current: usize, total: usize, phase: &str) {
    let _ = app.emit(
        "battery-catalog-progress",
        ProgressPayload {
            current,
            total,
            phase: phase.to_string(),
        },
    );
}

fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Keep it simple / sortable; full RFC3339 not required.
    format!("{secs}")
}
