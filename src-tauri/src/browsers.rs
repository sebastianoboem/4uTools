use adb_bridge::AdbBridge;
use serde::Serialize;
use std::collections::HashMap;

const VIEW_HTTPS: &str = "cmd package resolve-activity --brief -a android.intent.action.VIEW -c android.intent.category.BROWSABLE -d https://example.com";
const QUERY_HTTPS: &str = "cmd package query-activities -a android.intent.action.VIEW -c android.intent.category.BROWSABLE -d https://example.com";
const CLEARABLE: &[&str] = &[
    "com.android.chrome",
    "com.chrome.beta",
    "org.mozilla.firefox",
    "com.sec.android.app.sbrowser",
    "com.mi.globalbrowser",
];

#[derive(Debug, Clone, Serialize)]
pub struct BrowserCheck {
    pub sdk: u32,
    pub home: Option<String>,
    pub default_browser: Option<String>,
    pub browsers: Vec<BrowserInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub package: String,
    pub sites: Vec<String>,
    pub blocked: bool,
}

#[derive(Debug, Clone, Default)]
struct ParsedBrowser {
    sites: Vec<String>,
    notifications_off: bool,
}

pub fn check(bridge: &AdbBridge) -> Result<BrowserCheck, String> {
    let sdk = bridge
        .get_device_property("ro.build.version.sdk")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let home = uninstall_engine::default_home(bridge);
    let default_browser = bridge
        .shell(VIEW_HTTPS)
        .ok()
        .and_then(|out| uninstall_engine::component_package_from_output(&out));
    let mut packages: Vec<String> = bridge
        .shell(QUERY_HTTPS)
        .ok()
        .map(|text| package_scanner::parse_component_packages(&text))
        .unwrap_or_default()
        .into_iter()
        .filter(|package| safe_package(package))
        .collect();
    if let Some(package) = default_browser.clone() {
        if safe_package(&package) && !packages.iter().any(|name| name == &package) {
            packages.push(package);
        }
    }
    packages = include_installed(packages, &installed_clearable(bridge));

    let channels = bridge
        .shell("dumpsys notification --noredact")
        .ok()
        .map(|text| parse_site_channels(&text))
        .unwrap_or_default();

    let mut browsers = Vec::with_capacity(packages.len());
    for package in packages {
        let parsed = channels.get(&package).cloned().unwrap_or_default();
        let mut blocked = parsed.notifications_off;
        if sdk >= 33 {
            if let Ok(dump) = bridge.shell(&format!("dumpsys package {package}")) {
                if parse_notifications_granted(&dump) == Some(false) {
                    blocked = true;
                }
            }
        }
        browsers.push(BrowserInfo {
            package,
            sites: parsed.sites,
            blocked,
        });
    }

    Ok(BrowserCheck {
        sdk,
        home,
        default_browser,
        browsers,
    })
}

fn installed_clearable(bridge: &AdbBridge) -> Vec<String> {
    let script = CLEARABLE
        .iter()
        .map(|package| format!("pm path {package} >/dev/null 2>&1 && echo {package}"))
        .collect::<Vec<_>>()
        .join("; ");
    bridge
        .shell(&script)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| CLEARABLE.contains(line))
        .map(str::to_string)
        .collect()
}

fn include_installed(mut packages: Vec<String>, present: &[String]) -> Vec<String> {
    for package in CLEARABLE {
        if present.iter().any(|name| name == package) && !packages.iter().any(|name| name == *package) {
            packages.push((*package).to_string());
        }
    }
    packages.sort();
    packages.dedup();
    packages
}

fn safe_package(package: &str) -> bool {
    package.contains('.')
        && package.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn parse_site_channels(text: &str) -> HashMap<String, ParsedBrowser> {
    let mut map: HashMap<String, ParsedBrowser> = HashMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some((package, uid, off)) = parse_app_settings(line) {
            if uid < 100_000 && safe_package(package) {
                if off {
                    map.entry(package.to_string())
                        .or_default()
                        .notifications_off = true;
                } else {
                    map.entry(package.to_string()).or_default();
                }
                current = Some(package.to_string());
            } else {
                current = None;
            }
            continue;
        }
        let Some(package) = current.clone() else {
            continue;
        };
        if line.contains("mDeleted=true") || channel_importance_zero(line) {
            continue;
        }
        let Some(origin) = site_origin(line) else {
            continue;
        };
        let entry = map.entry(package).or_default();
        if !entry.sites.iter().any(|site| site == origin) {
            entry.sites.push(origin.to_string());
        }
    }
    for entry in map.values_mut() {
        entry.sites.sort();
    }
    map
}

fn parse_app_settings(line: &str) -> Option<(&str, u32, bool)> {
    let rest = line.trim().strip_prefix("AppSettings:")?.trim();
    let mut parts = rest.split_whitespace();
    let package = parts.next()?;
    let uid = parts
        .next()?
        .trim_matches(|c| c == '(' || c == ')')
        .parse()
        .ok()?;
    let off = rest
        .split_whitespace()
        .any(|word| word == "importance=NONE");
    Some((package, uid, off))
}

fn site_origin(line: &str) -> Option<&str> {
    let rest = line.split_once("mId='web:")?.1;
    let id = rest.split_once('\'')?.0;
    let origin = id.split(';').next().unwrap_or(id);
    if origin.is_empty() {
        None
    } else {
        Some(origin)
    }
}

fn channel_importance_zero(line: &str) -> bool {
    let Some(rest) = line.split_once("mImportance=").map(|(_, rest)| rest) else {
        return false;
    };
    rest.starts_with('0')
        && match rest.as_bytes().get(1) {
            Some(byte) => !byte.is_ascii_digit(),
            None => true,
        }
}

/// First `POST_NOTIFICATIONS: granted=` after `User 0:`.
pub fn parse_notifications_granted(text: &str) -> Option<bool> {
    let mut seen_user = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("User 0:") {
            seen_user = true;
        }
        if !seen_user {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("android.permission.POST_NOTIFICATIONS:") else {
            continue;
        };
        if rest.contains("granted=true") {
            return Some(true);
        }
        if rest.contains("granted=false") {
            return Some(false);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANNELS: &str = "\
      AppSettings: com.android.chrome (10156) importance=DEFAULT userSet=true\n\
        NotificationChannel{mId='web:https://www.google.com;13431803788360931', mName=www.google.com, mImportance=3, mDeleted=true, mGroup='sites'}\n\
        NotificationChannel{mId='web:https://news.example;99', mName=news, mImportance=3, mDeleted=false, mGroup='sites'}\n\
        NotificationChannel{mId='web:https://quiet.example;1', mName=quiet, mImportance=0, mDeleted=false}\n\
        NotificationChannel{mId='misc', mImportance=3, mDeleted=false}\n\
      AppSettings: com.android.chrome (1010156) importance=DEFAULT\n\
        NotificationChannel{mId='web:https://work.example;1', mImportance=3, mDeleted=false}\n\
      AppSettings: com.sec.android.app.sbrowser (10100) importance=NONE userSet=true\n\
        NotificationChannel{mId='web:https://samsung.example;2', mImportance=4, mDeleted=false}\n";

    #[test]
    fn installed_clearable_browsers_join_the_default() {
        let present = [
            "com.android.chrome".to_string(),
            "org.mozilla.firefox".to_string(),
            "com.mi.globalbrowser".to_string(),
        ];
        let packages = include_installed(vec!["com.android.chrome".to_string()], &present);
        assert_eq!(
            packages,
            vec![
                "com.android.chrome".to_string(),
                "com.mi.globalbrowser".to_string(),
                "org.mozilla.firefox".to_string(),
            ]
        );
    }

    #[test]
    fn site_channels_skip_deleted_muted_other_user_and_non_sites() {
        let parsed = parse_site_channels(CHANNELS);
        let chrome = parsed.get("com.android.chrome").unwrap();
        assert_eq!(chrome.sites, vec!["https://news.example".to_string()]);
        assert!(!chrome.notifications_off);
        let samsung = parsed.get("com.sec.android.app.sbrowser").unwrap();
        assert_eq!(samsung.sites, vec!["https://samsung.example".to_string()]);
        assert!(samsung.notifications_off);
        assert!(!parsed.contains_key("com.other"));
    }

    #[test]
    fn notifications_granted_uses_the_first_user_zero_line() {
        let dump = "\
User 10:\n\
        android.permission.POST_NOTIFICATIONS: granted=true, flags=[ USER_SET]\n\
    User 0: ceDataInode=5408 installed=true\n\
      runtime permissions:\n\
        android.permission.POST_NOTIFICATIONS: granted=false, flags=[ USER_SET]\n\
    User 0:\n\
        android.permission.POST_NOTIFICATIONS: granted=true, flags=[ USER_SET]\n";
        assert_eq!(parse_notifications_granted(dump), Some(false));
        assert_eq!(parse_notifications_granted("no user"), None);
    }
}
