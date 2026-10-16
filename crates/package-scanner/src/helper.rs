use adb_bridge::AdbBridge;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

const REMOTE_JAR: &str = "/data/local/tmp/aac-scan.jar";

pub struct HelperApp {
    pub package_name: String,
    pub label: Option<String>,
    pub author: Option<String>,
    pub icon_png: Option<Vec<u8>>,
}

/// Pushes the bundled jar, streams one stdout line per app, then deletes the jar.
/// Any failure yields no lines; the caller falls back to on-host APK parsing.
pub fn run(bridge: &AdbBridge, user_only: bool, tx: Sender<String>) {
    let Ok(jar) = write_temp_jar() else {
        return;
    };
    let pushed = bridge.push(&jar, REMOTE_JAR).is_ok();
    let _ = std::fs::remove_file(&jar);
    if !pushed {
        return;
    }
    let flag = if user_only { " -3" } else { "" };
    let cmd = format!(
        "CLASSPATH={REMOTE_JAR} timeout 60 app_process / ScanHelper{flag}; rm -f {REMOTE_JAR}"
    );
    let _ = bridge.shell_lines(&cmd, |line| {
        let _ = tx.send(line.to_string());
    });
}

fn write_temp_jar() -> std::io::Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("aac-scan-{}.jar", std::process::id()));
    std::fs::write(&path, include_bytes!("../helper/scan-helper.jar"))?;
    Ok(path)
}

pub fn parse_line(line: &str) -> Option<HelperApp> {
    let mut parts = line.split('\t');
    let package_name = parts.next()?.trim();
    if !is_package_token(package_name) {
        return None;
    }
    let label = decode_text(parts.next()?);
    let author = decode_text(parts.next()?);
    let icon_png = decode_bytes(parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    Some(HelperApp {
        package_name: package_name.to_string(),
        label,
        author,
        icon_png,
    })
}

fn decode_text(field: &str) -> Option<String> {
    let text = String::from_utf8(decode_bytes(field)?).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn is_package_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && name.chars().any(|c| c.is_ascii_alphabetic())
}

fn decode_bytes(field: &str) -> Option<Vec<u8>> {
    if field.is_empty() {
        return None;
    }
    STANDARD
        .decode(field)
        .ok()
        .filter(|bytes| !bytes.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_reads_label_author_and_icon() {
        let line = format!(
            "com.whatsapp\t{}\t{}\t{}",
            STANDARD.encode("WhatsApp"),
            STANDARD.encode("WhatsApp LLC"),
            STANDARD.encode(b"\x89PNG")
        );
        let app = parse_line(&line).unwrap();
        assert_eq!(app.package_name, "com.whatsapp");
        assert_eq!(app.label.as_deref(), Some("WhatsApp"));
        assert_eq!(app.author.as_deref(), Some("WhatsApp LLC"));
        assert_eq!(app.icon_png.unwrap(), b"\x89PNG");
    }

    #[test]
    fn parse_line_allows_empty_fields_and_rejects_garbage() {
        let app = parse_line("com.android.settings\t\t\t").unwrap();
        assert!(app.label.is_none() && app.author.is_none() && app.icon_png.is_none());
        assert!(parse_line("android\t\t\t").is_some());
        assert!(parse_line("not a package").is_none());
        assert!(parse_line("").is_none());
        assert!(parse_line("com.foo\tonly-one").is_none());
        assert!(parse_line("com.foo\ta\tb\tc\textra").is_none());
    }
}
