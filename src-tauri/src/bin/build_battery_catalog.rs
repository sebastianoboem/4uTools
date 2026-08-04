//! Offline builder: `cargo run -p fourutools --bin build-battery-catalog`
//! Writes `resources/battery-catalog.json` in the repo.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Minimal AppHandle-less sync for CI/dev: reuse the same scrape helpers via a local copy path.
    match run() {
        Ok(n) => {
            eprintln!("OK: {n} entries written");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<usize, String> {
    use regex::Regex;
    use reqwest::blocking::Client;
    use serde_json::json;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const SITEMAP: &str = "https://andreagaleazzi.com/schede-tecniche-sitemap.xml";
    const UA: &str = "4uTools/1.3.2 battery-catalog-builder";
    const WORKERS: usize = 8;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources/battery-catalog.json");

    let client = Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    eprintln!("Fetching sitemap…");
    let sitemap = client
        .get(SITEMAP)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| e.to_string())?;

    let re_loc =
        Regex::new(r"<loc>(https://andreagaleazzi\.com/schede-tecniche/[^<]+)</loc>").unwrap();
    let mut urls: Vec<String> = re_loc
        .captures_iter(&sitemap)
        .map(|c| c[1].trim().to_string())
        .filter(|u| u.ends_with('/'))
        .collect();
    urls.sort();
    urls.dedup();
    let total = urls.len();
    eprintln!("Found {total} schede");

    let log_re = Regex::new(r"console\.log\('(\{.*?\})'\)").unwrap();
    let entries = Arc::new(Mutex::new(Vec::new()));
    let done = Arc::new(AtomicUsize::new(0));
    let chunk = (total + WORKERS - 1) / WORKERS;

    thread::scope(|scope| {
        for part in urls.chunks(chunk.max(1)) {
            let part = part.to_vec();
            let client = client.clone();
            let log_re = log_re.clone();
            let entries = Arc::clone(&entries);
            let done = Arc::clone(&done);
            scope.spawn(move || {
                for url in part {
                    if let Ok(html) = client.get(&url).send().and_then(|r| r.error_for_status()).and_then(|r| r.text()) {
                        if let Some(caps) = log_re.captures(&html) {
                            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&caps[1]) {
                                let mah = match obj.get("batteria") {
                                    Some(serde_json::Value::String(s)) => {
                                        s.parse::<u32>().unwrap_or(0)
                                    }
                                    Some(serde_json::Value::Number(n)) => {
                                        n.as_u64().unwrap_or(0) as u32
                                    }
                                    _ => 0,
                                };
                                if mah > 0 {
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
                                    if let Ok(mut g) = entries.lock() {
                                        g.push(json!({
                                            "slug": slug,
                                            "marca": marca,
                                            "nome": nome,
                                            "batteriaMah": mah,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % 25 == 0 || n == total {
                        eprintln!("… {n}/{total}");
                    }
                    thread::sleep(Duration::from_millis(40));
                }
            });
        }
    });

    let mut list = entries.lock().map_err(|e| e.to_string())?.clone();
    list.sort_by(|a, b| {
        a.get("slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("slug").and_then(|v| v.as_str()).unwrap_or(""))
    });
    list.dedup_by(|a, b| a.get("slug") == b.get("slug"));

    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();

    let file = json!({
        "version": 1,
        "updatedAt": updated_at,
        "source": "andreagaleazzi.com",
        "entries": list,
    });

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&out, serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    eprintln!("Wrote {}", out.display());
    Ok(list.len())
}
