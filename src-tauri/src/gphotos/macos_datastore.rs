//! Direct WKWebsiteDataStore cookie ops — avoid wry's per-cookie main-thread wait
//! (`wait_for_blocking_operation`, 1s each) that freezes the UI during import.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use block2::RcBlock;
use cookie::Cookie;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_foundation::{
    ns_string, NSArray, NSDate, NSHTTPCookie, NSHTTPCookieDomain, NSHTTPCookieExpires,
    NSHTTPCookieMaximumAge, NSHTTPCookieName, NSHTTPCookiePath, NSHTTPCookiePropertyKey,
    NSHTTPCookieSecure, NSHTTPCookieValue, NSHTTPCookieVersion, NSMutableDictionary, NSSet,
    NSString, NSUUID,
};
use objc2_web_kit::WKWebsiteDataStore;
use tauri::AppHandle;

use super::webview::PHOTOS_DATA_STORE_ID;

const OP_TIMEOUT: Duration = Duration::from_secs(20);

fn photos_store(mtm: MainThreadMarker) -> objc2::rc::Retained<WKWebsiteDataStore> {
    unsafe {
        let version = objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion();
        if version.majorVersion >= 14 {
            let identifier = NSUUID::from_bytes(PHOTOS_DATA_STORE_ID);
            WKWebsiteDataStore::dataStoreForIdentifier(&identifier, mtm)
        } else {
            WKWebsiteDataStore::defaultDataStore(mtm)
        }
    }
}

unsafe fn cookie_into_wkwebview(cookie: &Cookie<'_>) -> objc2::rc::Retained<NSHTTPCookie> {
    let nstring_true: &'static NSString = ns_string!("TRUE");
    let nstring_false: &'static NSString = ns_string!("FALSE");
    let nstring_0: &'static NSString = ns_string!("0");
    let nstring_1: &'static NSString = ns_string!("1");

    let name = NSString::from_str(cookie.name());
    let value = NSString::from_str(cookie.value());
    let path = cookie.path().map_or_else(NSString::new, NSString::from_str);
    // Preserve a leading dot so NSHTTPCookie is a domain cookie (subdomains).
    let domain = match cookie.domain() {
        Some(d) if !d.is_empty() && !d.starts_with('.') && d.contains('.') => {
            NSString::from_str(&format!(".{d}"))
        }
        Some(d) => NSString::from_str(d),
        None => NSString::new(),
    };

    let properties: objc2::rc::Retained<NSMutableDictionary<NSHTTPCookiePropertyKey, AnyObject>> =
        NSMutableDictionary::from_slices(
            &[
                NSHTTPCookieName,
                NSHTTPCookieValue,
                NSHTTPCookiePath,
                NSHTTPCookieDomain,
            ],
            &[&name, &value, &path, &domain],
        );

    if let Some(max_age_) = cookie.max_age() {
        let max_age = NSString::from_str(&max_age_.whole_seconds().to_string());
        properties.insert(NSHTTPCookieMaximumAge, &*max_age);
        properties.insert(NSHTTPCookieVersion, nstring_1);
    } else if let Some(dt) = cookie.expires_datetime() {
        let expires = NSDate::dateWithTimeIntervalSince1970(dt.unix_timestamp() as f64);
        properties.insert(NSHTTPCookieExpires, &*expires);
        properties.insert(NSHTTPCookieVersion, nstring_0);
    }

    if let Some(secure) = cookie.secure() {
        let secure = if secure { nstring_true } else { nstring_false };
        properties.insert(NSHTTPCookieSecure, secure);
    }

    if let Some(http_only) = cookie.http_only() {
        let http_only = if http_only {
            nstring_true
        } else {
            nstring_false
        };
        properties.insert(ns_string!("HttpOnly"), http_only);
    }

    if let Some(same_site) = cookie.same_site() {
        let key = ns_string!("SameSite");
        match same_site {
            cookie::SameSite::Lax => {
                properties.insert(key, ns_string!("lax"));
            }
            cookie::SameSite::Strict => {
                properties.insert(key, ns_string!("strict"));
            }
            cookie::SameSite::None => {}
        };
    }

    NSHTTPCookie::cookieWithProperties(&properties)
        .expect("failed to create NSHTTPCookie from session cookie")
}

fn run_on_main<F>(app: &AppHandle, f: F) -> Result<(), String>
where
    F: FnOnce() + Send + 'static,
{
    app.run_on_main_thread(f).map_err(|e| e.to_string())
}

pub async fn clear_all(app: &AppHandle) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_err = tx.clone();
    run_on_main(app, move || {
        let send = |result: Result<(), String>| {
            if let Ok(mut slot) = tx.lock() {
                if let Some(tx) = slot.take() {
                    let _ = tx.send(result);
                }
            }
        };
        let Some(mtm) = MainThreadMarker::new() else {
            send(Err("clear cookie store: not on main thread".into()));
            return;
        };
        unsafe {
            let store = photos_store(mtm);
            let types: objc2::rc::Retained<NSSet<NSString>> =
                WKWebsiteDataStore::allWebsiteDataTypes(mtm);
            let date = NSDate::dateWithTimeIntervalSince1970(0.0);
            let tx = tx_err.clone();
            let handler = RcBlock::new(move || {
                if let Ok(mut slot) = tx.lock() {
                    if let Some(tx) = slot.take() {
                        let _ = tx.send(Ok(()));
                    }
                }
            });
            store.removeDataOfTypes_modifiedSince_completionHandler(&types, &date, &handler);
        }
    })?;
    match tokio::time::timeout(OP_TIMEOUT, rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("clear cookie store: canale chiuso".into()),
        Err(_) => Err("clear cookie store: timeout".into()),
    }
}

pub async fn apply_cookies(app: &AppHandle, cookies: Vec<Cookie<'static>>) -> Result<usize, String> {
    let total = cookies.len();
    if total == 0 {
        return Ok(0);
    }
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<usize, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_main = tx.clone();
    run_on_main(app, move || {
        let send = |result: Result<usize, String>| {
            if let Ok(mut slot) = tx_main.lock() {
                if let Some(tx) = slot.take() {
                    let _ = tx.send(result);
                }
            }
        };
        let Some(mtm) = MainThreadMarker::new() else {
            send(Err("apply cookies: not on main thread".into()));
            return;
        };
        unsafe {
            let store = photos_store(mtm);
            let http = store.httpCookieStore();
            let mut wk_cookies = Vec::with_capacity(cookies.len());
            for cookie in &cookies {
                wk_cookies.push(cookie_into_wkwebview(cookie));
            }
            let array = NSArray::from_retained_slice(&wk_cookies);
            let tx_done = tx.clone();
            let handler = RcBlock::new(move || {
                if let Ok(mut slot) = tx_done.lock() {
                    if let Some(tx) = slot.take() {
                        let _ = tx.send(Ok(total));
                    }
                }
            });
            http.setCookies_completionHandler(&array, Some(&handler));
        }
    })?;
    match tokio::time::timeout(OP_TIMEOUT, rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("apply cookies: canale chiuso".into()),
        Err(_) => Err("apply cookies: timeout".into()),
    }
}

pub fn clear_all_fire_and_forget(app: &AppHandle) {
    let _ = run_on_main(app, || {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        unsafe {
            let store = photos_store(mtm);
            let types = WKWebsiteDataStore::allWebsiteDataTypes(mtm);
            let date = NSDate::dateWithTimeIntervalSince1970(0.0);
            let handler = RcBlock::new(|| {});
            store.removeDataOfTypes_modifiedSince_completionHandler(&types, &date, &handler);
        }
    });
}
