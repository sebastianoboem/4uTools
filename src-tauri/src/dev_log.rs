use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

// Release GitHub: false. scripts/check-dev-log.mjs rifiuta il manifest se resta true.
pub const ENABLED: bool = false;

const MAX_BYTES: u64 = 2 * 1024 * 1024;

static TARGET: OnceLock<LogTarget> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

struct LogTarget {
    dir: PathBuf,
    share: bool,
}

pub fn init() {
    if !ENABLED {
        return;
    }
    let target = target();
    record(&format!(
        "start version={} path={} share={}",
        env!("CARGO_PKG_VERSION"),
        target.dir.display(),
        if target.share { "yes" } else { "no" }
    ));
}

pub fn record(line: &str) {
    if !ENABLED {
        return;
    }
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = append_limited(&target().dir, line, MAX_BYTES);
}

pub fn command(name: &str, err: Option<&str>) {
    match err {
        Some(e) => record(&format!("{name} ERR {e}")),
        None => record(&format!("{name} ok")),
    }
}

pub fn logged<T, E: std::fmt::Display>(
    name: &str,
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let result = f();
    log_result(name, &result);
    result
}

pub fn log_result<T, E: std::fmt::Display>(name: &str, result: &Result<T, E>) {
    match result {
        Ok(_) => command(name, None),
        Err(e) => command(name, Some(&e.to_string())),
    }
}

fn target() -> &'static LogTarget {
    TARGET.get_or_init(|| {
        for (dir, share) in candidates() {
            if probe_writable(&dir) {
                return LogTarget { dir, share };
            }
        }
        LogTarget {
            dir: PathBuf::from("."),
            share: false,
        }
    })
}

fn candidates() -> Vec<(PathBuf, bool)> {
    let mut dirs = Vec::new();
    #[cfg(windows)]
    {
        dirs.push((PathBuf::from(r"\\psf\aac-logs"), true));
        dirs.push((PathBuf::from(r"\\Mac\psf\aac-logs"), true));
    }
    if let Some(data) = dirs::data_dir() {
        dirs.push((data.join("AndroidAdwareCleaner"), false));
    }
    dirs
}

fn probe_writable(dir: &Path) -> bool {
    let dir = dir.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ok = std::fs::create_dir_all(&dir).is_ok()
            && OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("dev.log"))
                .is_ok();
        let _ = tx.send(ok);
    });
    rx.recv_timeout(Duration::from_millis(500)).unwrap_or(false)
}

fn append_limited(dir: &Path, line: &str, max_bytes: u64) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("dev.log");
    if path.metadata().map(|m| m.len()).unwrap_or(0) > max_bytes {
        let backup = dir.join("dev.log.1");
        let _ = std::fs::remove_file(&backup);
        let _ = std::fs::rename(&path, &backup);
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let line = line.replace(['\r', '\n'], " ");
    writeln!(file, "{stamp} {line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_adb_stream_is_a_count() {
        let line = adb_bridge::format_dev_stream("stdout", &"pkg\n".repeat(2000));
        assert!(line.contains("bytes"), "{line}");
        assert!(line.contains("lines"), "{line}");
        assert!(!line.contains("pkg"));
    }

    #[test]
    fn short_adb_stream_keeps_text() {
        assert_eq!(
            adb_bridge::format_dev_stream("stderr", "device offline"),
            "stderr device offline"
        );
        assert_eq!(adb_bridge::format_dev_stream("stdout", ""), "");
    }

    #[test]
    fn rotates_when_the_file_passes_the_limit() {
        let dir = std::env::temp_dir().join(format!("aac-dev-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        append_limited(&dir, &"x".repeat(80), 100).unwrap();
        append_limited(&dir, "next", 100).unwrap();
        let backup = std::fs::read_to_string(dir.join("dev.log.1")).unwrap();
        let current = std::fs::read_to_string(dir.join("dev.log")).unwrap();
        assert!(backup.contains(&"x".repeat(80)));
        assert!(current.contains("next"));
        assert!(!current.contains(&"x".repeat(80)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
