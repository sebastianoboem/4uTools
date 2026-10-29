use std::collections::BTreeMap;

use cookie::{Cookie, Expiration, SameSite};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use time::OffsetDateTime;

use super::engine::eval_value;
use super::state::{photos, GphotosState};
use super::webview::reload_photos;

const AUTH_COOKIE_NAMES: &[&str] = &[
    "SID",
    "HSID",
    "SSID",
    "APISID",
    "SAPISID",
    "__Secure-1PSID",
    "__Secure-3PSID",
    "__Secure-1PAPISID",
    "__Secure-3PAPISID",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProfile {
    pub id: String,
    pub name: String,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStoreData {
    pub profiles: Vec<SavedProfile>,
    #[serde(default)]
    pub active_profile_id: Option<String>,
}

impl Default for ProfileStoreData {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            active_profile_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredCookie {
    pub(super) name: String,
    pub(super) value: String,
    #[serde(default)]
    pub(super) domain: Option<String>,
    #[serde(default)]
    pub(super) path: Option<String>,
    #[serde(default)]
    pub(super) secure: Option<bool>,
    #[serde(default)]
    pub(super) http_only: Option<bool>,
    #[serde(default)]
    pub(super) expiration_date: Option<f64>,
    #[serde(default)]
    pub(super) same_site: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WebStorage {
    #[serde(default)]
    local: BTreeMap<String, Option<String>>,
    #[serde(default)]
    session: BTreeMap<String, Option<String>>,
}

fn store_path(state: &GphotosState) -> std::path::PathBuf {
    state.root.join("profiles.json")
}

fn profile_dir(state: &GphotosState) -> std::path::PathBuf {
    state.root.join("profile-data")
}

pub fn load_store(state: &GphotosState) -> ProfileStoreData {
    let Ok(text) = std::fs::read_to_string(store_path(state)) else {
        return ProfileStoreData::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_store(state: &GphotosState, store: &ProfileStoreData) -> Result<(), String> {
    if let Some(parent) = store_path(state).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(store_path(state), text).map_err(|e| e.to_string())
}

fn cookies_path(state: &GphotosState, id: &str) -> std::path::PathBuf {
    profile_dir(state).join(format!("{id}-cookies.json"))
}

fn storage_path(state: &GphotosState, id: &str) -> std::path::PathBuf {
    profile_dir(state).join(format!("{id}-storage.json"))
}

fn auth_count(cookies: &[StoredCookie]) -> usize {
    cookies
        .iter()
        .filter(|cookie| AUTH_COOKIE_NAMES.contains(&cookie.name.as_str()))
        .count()
}

fn skip_domain(domain: Option<&str>) -> bool {
    domain
        .unwrap_or("")
        .contains("usercontent.google.com")
}

fn same_site_from(raw: Option<&str>) -> Option<SameSite> {
    match raw.unwrap_or("").to_ascii_lowercase().as_str() {
        "strict" => Some(SameSite::Strict),
        "lax" => Some(SameSite::Lax),
        "none" | "no_restriction" => Some(SameSite::None),
        _ => None,
    }
}

fn same_site_to(value: Option<SameSite>) -> Option<String> {
    match value {
        Some(SameSite::Strict) => Some("strict".into()),
        Some(SameSite::Lax) => Some("lax".into()),
        Some(SameSite::None) => Some("no_restriction".into()),
        None => None,
    }
}

fn to_runtime_cookie(cookie: &StoredCookie) -> Cookie<'static> {
    let mut built = Cookie::new(cookie.name.clone(), cookie.value.clone());
    if !cookie.name.starts_with("__Host-") {
        if let Some(domain) = cookie.domain.as_deref() {
            let domain = if domain.starts_with('.') || cookie.name.starts_with("__Secure-") {
                domain.to_string()
            } else if domain.contains('.') {
                format!(".{domain}")
            } else {
                domain.to_string()
            };
            built.set_domain(domain);
        }
    }
    built.set_path(cookie.path.clone().unwrap_or_else(|| "/".into()));
    built.set_secure(cookie.secure.unwrap_or(true));
    built.set_http_only(cookie.http_only.unwrap_or(false));
    if let Some(same) = same_site_from(cookie.same_site.as_deref()) {
        built.set_same_site(same);
    }
    if let Some(exp) = cookie.expiration_date {
        if let Ok(when) = OffsetDateTime::from_unix_timestamp(exp as i64) {
            built.set_expires(Expiration::DateTime(when));
        }
    }
    built
}

fn from_runtime_cookie(cookie: &Cookie<'_>) -> Option<StoredCookie> {
    let domain = cookie.domain().map(str::to_string);
    if skip_domain(domain.as_deref()) {
        return None;
    }
    let expiration_date = cookie
        .expires_datetime()
        .and_then(|when| when.unix_timestamp().try_into().ok())
        .map(|secs: i64| secs as f64);
    Some(StoredCookie {
        name: cookie.name().to_string(),
        value: cookie.value().to_string(),
        domain,
        path: cookie.path().map(str::to_string),
        secure: cookie.secure(),
        http_only: cookie.http_only(),
        expiration_date,
        same_site: same_site_to(cookie.same_site()),
    })
}

fn load_cookies(state: &GphotosState, id: &str) -> Vec<StoredCookie> {
    let Ok(text) = std::fs::read_to_string(cookies_path(state, id)) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn load_storage(state: &GphotosState, id: &str) -> WebStorage {
    let Ok(text) = std::fs::read_to_string(storage_path(state, id)) else {
        return WebStorage::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

async fn read_storage(app: &tauri::AppHandle) -> Result<WebStorage, String> {
    let webview = photos(app)?;
    let value = eval_value(
        &webview,
        r#"({
          local: Object.fromEntries(Object.keys(localStorage).map((k) => [k, localStorage.getItem(k)])),
          session: Object.fromEntries(Object.keys(sessionStorage).map((k) => [k, sessionStorage.getItem(k)]))
        })"#,
    )
    .await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

async fn write_storage(app: &tauri::AppHandle, storage: &WebStorage) -> Result<(), String> {
    let webview = photos(app)?;
    let payload = serde_json::to_string(storage).map_err(|e| e.to_string())?;
    let script = format!(
        r#"(function(data) {{
          localStorage.clear();
          sessionStorage.clear();
          for (const [k, v] of Object.entries(data.local || {{}})) {{
            if (typeof v === "string") localStorage.setItem(k, v);
          }}
          for (const [k, v] of Object.entries(data.session || {{}})) {{
            if (typeof v === "string") sessionStorage.setItem(k, v);
          }}
        }})({payload})"#
    );
    webview.eval(script).map_err(|e| e.to_string())
}

pub async fn snapshot_cookies(app: &tauri::AppHandle) -> Result<Vec<StoredCookie>, String> {
    let webview = photos(app)?;
    let cookies = webview.cookies().map_err(|e| e.to_string())?;
    Ok(cookies.iter().filter_map(from_runtime_cookie).collect())
}

pub fn apply_stored_cookies(app: &tauri::AppHandle, cookies: &[StoredCookie]) -> Result<usize, String> {
    let webview = photos(app)?;
    let mut imported = 0;
    for cookie in cookies {
        if skip_domain(cookie.domain.as_deref()) {
            continue;
        }
        let built = to_runtime_cookie(cookie);
        if webview.set_cookie(built).is_ok() {
            imported += 1;
        }
    }
    Ok(imported)
}

pub async fn save_named_profile(app: &tauri::AppHandle, name: &str) -> Result<ProfileStoreData, String> {
    let state = app.state::<GphotosState>();
    let cookies = snapshot_cookies(app).await?;
    if auth_count(&cookies) < 3 {
        return Err("Accedi a Google Foto prima di salvare la sessione.".into());
    }
    let storage = read_storage(app).await.unwrap_or_default();
    let profile = SavedProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                "Profilo senza nome".into()
            } else {
                trimmed.to_string()
            }
        },
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0),
    };
    std::fs::create_dir_all(profile_dir(&state)).map_err(|e| e.to_string())?;
    let cookie_text = serde_json::to_string_pretty(&cookies).map_err(|e| e.to_string())?;
    std::fs::write(cookies_path(&state, &profile.id), cookie_text).map_err(|e| e.to_string())?;
    let storage_text = serde_json::to_string(&storage).map_err(|e| e.to_string())?;
    std::fs::write(storage_path(&state, &profile.id), storage_text).map_err(|e| e.to_string())?;
    let mut store = load_store(&state);
    store.profiles.push(profile.clone());
    store.active_profile_id = Some(profile.id);
    save_store(&state, &store)?;
    Ok(store)
}

pub fn delete_named_profile(app: &tauri::AppHandle, profile_id: &str) -> Result<ProfileStoreData, String> {
    let state = app.state::<GphotosState>();
    let mut store = load_store(&state);
    let before = store.profiles.len();
    store.profiles.retain(|profile| profile.id != profile_id);
    if store.profiles.len() == before {
        return Err("Profilo non trovato.".into());
    }
    if store.active_profile_id.as_deref() == Some(profile_id) {
        store.active_profile_id = None;
    }
    let _ = std::fs::remove_file(cookies_path(&state, profile_id));
    let _ = std::fs::remove_file(storage_path(&state, profile_id));
    save_store(&state, &store)?;
    Ok(store)
}

pub async fn switch_profile(app: &tauri::AppHandle, profile_id: Option<String>) -> Result<ProfileStoreData, String> {
    let state = app.state::<GphotosState>();
    let mut store = load_store(&state);
    if let Some(id) = profile_id.as_deref() {
        if !store.profiles.iter().any(|profile| profile.id == id) {
            return Err("Profilo non trovato.".into());
        }
    }
    let cookies = profile_id
        .as_deref()
        .map(|id| load_cookies(&state, id))
        .unwrap_or_default();
    if profile_id.is_some() && auth_count(&cookies) < 3 {
        return Err("Sessione salvata incompleta. Accedi di nuovo e salvala.".into());
    }
    let storage = profile_id
        .as_deref()
        .map(|id| load_storage(&state, id))
        .unwrap_or_default();
    let webview = photos(app)?;
    webview.clear_all_browsing_data().map_err(|e| e.to_string())?;
    if profile_id.is_some() {
        apply_stored_cookies(app, &cookies)?;
        write_storage(app, &storage).await?;
    }
    store.active_profile_id = profile_id;
    save_store(&state, &store)?;
    reload_photos(app)?;
    Ok(store)
}
