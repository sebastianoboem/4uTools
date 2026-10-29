use std::path::PathBuf;
use std::time::{Duration, Instant};

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::CookieSameSite;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};

use super::profiles::{apply_stored_cookies, StoredCookie};
use super::state::push_log;
use super::webview::reload_photos;

const PHOTOS_URL: &str = "https://photos.google.com/";

fn browser_candidates() -> Vec<(&'static str, PathBuf)> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        paths.push((
            "Chrome",
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
        ));
        paths.push((
            "Chrome",
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
        ));
        paths.push((
            "Microsoft Edge",
            PathBuf::from("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
        ));
    }
    #[cfg(target_os = "windows")]
    {
        for root in ["C:\\Program Files", "C:\\Program Files (x86)"] {
            paths.push((
                "Chrome",
                PathBuf::from(root).join("Google\\Chrome\\Application\\chrome.exe"),
            ));
            paths.push((
                "Microsoft Edge",
                PathBuf::from(root).join("Microsoft\\Edge\\Application\\msedge.exe"),
            ));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for (name, path) in [
            ("Chrome", "/usr/bin/google-chrome"),
            ("Chrome", "/usr/bin/google-chrome-stable"),
            ("Chrome", "/usr/bin/chromium"),
            ("Chrome", "/usr/bin/chromium-browser"),
            ("Microsoft Edge", "/usr/bin/microsoft-edge"),
            ("Microsoft Edge", "/usr/bin/microsoft-edge-stable"),
        ] {
            paths.push((name, PathBuf::from(path)));
        }
    }
    paths
}

fn resolve_browser() -> Result<(&'static str, PathBuf), String> {
    browser_candidates()
        .into_iter()
        .find(|(_, path)| path.is_file())
        .ok_or_else(|| "Né Google Chrome né Microsoft Edge trovati. Installa uno dei due per il login.".into())
}

fn same_site_label(value: Option<CookieSameSite>) -> Option<String> {
    match value {
        Some(CookieSameSite::Strict) => Some("strict".into()),
        Some(CookieSameSite::Lax) => Some("lax".into()),
        Some(CookieSameSite::None) => Some("no_restriction".into()),
        None => None,
    }
}

pub async fn run_chrome_login(app: &AppHandle) -> Result<usize, String> {
    let (display, executable) = resolve_browser()?;
    let dir = std::env::temp_dir().join(format!("gfm-login-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let config = BrowserConfig::builder()
        .chrome_executable(&executable)
        .with_head()
        .user_data_dir(&dir)
        .disable_default_args()
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-sync")
        .build()
        .map_err(|e| e)?;
    let status = format!("Apertura di {display} per il login…");
    let _ = app.emit("gphotos-login", serde_json::json!({ "message": status }));
    push_log(app, "info", &status);

    let launched = Browser::launch(config).await.map_err(|e| e.to_string());
    let (mut browser, mut handler) = match launched {
        Ok(pair) => pair,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(err);
        }
    };
    let driver = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    let result = login_session(app, &browser, display).await;
    let _ = browser.close().await;
    driver.abort();
    let _ = browser.kill().await;
    let _ = std::fs::remove_dir_all(&dir);
    let count = result?;
    reload_photos(app)?;
    Ok(count)
}

async fn login_session(
    app: &AppHandle,
    browser: &chromiumoxide::Browser,
    display: &str,
) -> Result<usize, String> {
    let page = browser
        .new_page("about:blank")
        .await
        .map_err(|e| e.to_string())?;
    let _ = page
        .evaluate_on_new_document(
            "Object.defineProperty(navigator, 'webdriver', { get: () => undefined })",
        )
        .await;
    page.goto(PHOTOS_URL).await.map_err(|e| e.to_string())?;
    let hint = format!("Accedi al tuo account Google in {display}, poi torna qui.");
    let _ = app.emit("gphotos-login", serde_json::json!({ "message": hint }));

    let deadline = Instant::now() + Duration::from_secs(300);
    let mut logged_in = false;
    while Instant::now() < deadline {
        let url = page.url().await.ok().flatten().unwrap_or_default();
        if url.contains("signin/rejected") {
            return Err(format!(
                "Google ha rifiutato il login (browser automatizzato). Chiudi {display} e riprova."
            ));
        }
        if url.contains("photos.google.com") {
            let blocked = page
                .evaluate(
                    r#"(() => {
                      const text = document.body?.innerText ?? "";
                      return text.includes("non essere sicur") || text.includes("may not be secure") || text.includes("Impossibile eseguire l'accesso");
                    })()"#,
                )
                .await
                .ok()
                .and_then(|value| value.into_value::<bool>().ok())
                .unwrap_or(false);
            let has_main = page
                .evaluate(r#"document.querySelector('[role="main"]') !== null"#)
                .await
                .ok()
                .and_then(|value| value.into_value::<bool>().ok())
                .unwrap_or(false);
            if !blocked && has_main {
                logged_in = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if !logged_in {
        return Err("Login non completato in tempo. Riprova «Accedi con Chrome».".into());
    }

    let _ = app.emit(
        "gphotos-login",
        serde_json::json!({ "message": "Importazione sessione nell'app…" }),
    );
    let raw = page.get_cookies().await.map_err(|e| e.to_string())?;
    let cookies: Vec<StoredCookie> = raw
        .into_iter()
        .filter(|cookie| !cookie.domain.contains("usercontent.google.com"))
        .map(|cookie| StoredCookie {
            name: cookie.name,
            value: cookie.value,
            domain: Some(cookie.domain),
            path: Some(cookie.path),
            secure: Some(cookie.secure),
            http_only: Some(cookie.http_only),
            expiration_date: if cookie.expires > 0.0 {
                Some(cookie.expires)
            } else {
                None
            },
            same_site: same_site_label(cookie.same_site),
        })
        .collect();
    if cookies.is_empty() {
        return Err(format!(
            "Nessun cookie importato. Verifica di essere loggato in {display}."
        ));
    }
    apply_stored_cookies(app, &cookies)
}
