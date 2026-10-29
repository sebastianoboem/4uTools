mod arsc;
mod helper;

use adb_bridge::{AdbBridge, PackageInfo};
use arsc::{lookup_all_strings, lookup_string, parse_resource_ref};
use axmldecoder::{parse as parse_manifest, Node};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error(transparent)]
    Adb(#[from] adb_bridge::AdbError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPackage {
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
}

#[derive(Debug, Clone)]
pub enum ScanProgress {
    Started { total: usize },
    Package(ScannedPackage),
}

#[derive(Debug, Clone, Default)]
struct PackageDumpInfo {
    installer: Option<String>,
    signing_org: Option<String>,
    first_install_ms: Option<i64>,
}

enum LabelValue {
    Literal(String),
    Reference(u32),
}

struct AppMeta {
    label: Option<LabelValue>,
    icon_res: Option<u32>,
}

/// Enrichment is I/O-bound (adb round-trips), so the pool is larger than the
/// CPU count; it also caps concurrent adb commands.
const SCAN_THREADS: usize = 16;

pub struct PackageScanner {
    bridge: Arc<AdbBridge>,
}

impl PackageScanner {
    pub fn new(bridge: Arc<AdbBridge>) -> Self {
        Self { bridge }
    }

    pub fn scan(&self, user_only: bool) -> Result<Vec<ScannedPackage>, ScanError> {
        let mut results = Vec::new();
        self.scan_with_progress(user_only, |event| {
            if let ScanProgress::Package(pkg) = event {
                results.push(pkg);
            }
        })?;
        Ok(results)
    }

    pub fn scan_with_progress<F>(
        &self,
        user_only: bool,
        mut on_progress: F,
    ) -> Result<(), ScanError>
    where
        F: FnMut(ScanProgress) + Send,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let helper_bridge = Arc::clone(&self.bridge);
        let helper_thread = thread::spawn(move || helper::run(&helper_bridge, user_only, tx));

        let (packages, dump_info, admins, signals) = thread::scope(|scope| {
            let packages = scope.spawn(|| self.bridge.list_packages(user_only));
            let dump = scope.spawn(|| {
                exec_out_gz(&self.bridge, "dumpsys package")
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_default()
            });
            let admins = scope.spawn(|| self.load_admin_packages());
            let signals = scope.spawn(|| RiskSignals::load(&self.bridge));
            Ok::<_, ScanError>((
                packages.join().expect("list packages")?,
                parse_bulk_dumpsys(&dump.join().expect("dumpsys package")),
                admins.join().expect("device policy")?,
                signals.join().expect("risk signals"),
            ))
        })?;
        let now_ms = now_millis();
        on_progress(ScanProgress::Started {
            total: packages.len(),
        });

        let known: HashSet<&str> = packages
            .iter()
            .map(|pkg| pkg.package_name.as_str())
            .collect();
        let mut seen = HashSet::new();
        for line in rx {
            let Some(app) = helper::parse_line(&line) else {
                continue;
            };
            if !known.contains(app.package_name.as_str()) || !seen.insert(app.package_name.clone())
            {
                continue;
            }
            let Some(pkg) = packages
                .iter()
                .find(|pkg| pkg.package_name == app.package_name)
            else {
                continue;
            };
            let mut scanned = package_from_helper(pkg, &app, &dump_info, &admins);
            attach_risk(
                &mut scanned,
                dump_info.get(&pkg.package_name),
                &signals,
                now_ms,
            );
            on_progress(ScanProgress::Package(scanned));
        }
        let _ = helper_thread.join();

        let missing = missing_packages(&packages, &seen);
        if !missing.is_empty() {
            let on_progress = Arc::new(Mutex::new(on_progress));
            let bridge = Arc::clone(&self.bridge);
            let dump_info = Arc::new(dump_info);
            let admins = Arc::new(admins);
            let signals = Arc::new(signals);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(SCAN_THREADS)
                .build()
                .expect("failed to build scan thread pool");
            pool.install(|| {
                missing.par_iter().for_each(|pkg| {
                    let mut scanned = enrich_package(&bridge, pkg, &dump_info, &admins);
                    attach_risk(
                        &mut scanned,
                        dump_info.get(&pkg.package_name),
                        &signals,
                        now_ms,
                    );
                    if let Ok(mut cb) = on_progress.lock() {
                        cb(ScanProgress::Package(scanned));
                    }
                });
            });
        }

        Ok(())
    }

    fn load_admin_packages(&self) -> Result<HashSet<String>, ScanError> {
        let output = self
            .bridge
            .shell("dumpsys device_policy")
            .unwrap_or_default();
        let mut admins = HashSet::new();
        for line in output.lines() {
            if line.contains("admin=ComponentInfo{") {
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.find('}') {
                        let inner = &line[start + 1..end];
                        if let Some(slash) = inner.find('/') {
                            let pkg = &inner[..slash];
                            admins.insert(pkg.to_string());
                        }
                    }
                }
            }
        }
        Ok(admins)
    }
}

fn enrich_package(
    bridge: &AdbBridge,
    pkg: &PackageInfo,
    dump_info: &HashMap<String, PackageDumpInfo>,
    admins: &HashSet<String>,
) -> ScannedPackage {
    let dumpsys_info = dump_info
        .get(&pkg.package_name)
        .cloned()
        .unwrap_or_default();
    let installer = dumpsys_info.installer.clone();
    let author = dumpsys_info.signing_org.clone();

    let manifest_bytes = fetch_apk_entry(
        bridge,
        &pkg.package_name,
        pkg.apk_path.as_deref(),
        "AndroidManifest.xml",
    );
    let meta = manifest_bytes
        .as_deref()
        .map(parse_application_meta)
        .unwrap_or(AppMeta {
            label: None,
            icon_res: None,
        });

    let needs_arsc =
        matches!(meta.label, Some(LabelValue::Reference(_))) || meta.icon_res.is_some();
    let arsc_bytes = if needs_arsc {
        fetch_apk_entry(
            bridge,
            &pkg.package_name,
            pkg.apk_path.as_deref(),
            "resources.arsc",
        )
    } else {
        None
    };

    let apk_label = resolve_apk_label(&meta, arsc_bytes.as_deref());
    let label = apk_label
        .clone()
        .or_else(|| known_label(&pkg.package_name))
        .or_else(|| infer_label_from_package(&pkg.package_name));
    let icon_url = resolve_icon(
        bridge,
        &pkg.package_name,
        pkg.apk_path.as_deref(),
        &meta,
        arsc_bytes.as_deref(),
    );

    ScannedPackage {
        package_name: pkg.package_name.clone(),
        label,
        author,
        icon_url,
        is_system: pkg.is_system,
        installer,
        is_device_admin: admins.contains(&pkg.package_name),
        sideloaded: false,
        installed_recently: false,
        reads_screen: false,
        draws_over: false,
        no_launcher_icon: false,
    }
}

fn package_from_helper(
    pkg: &PackageInfo,
    app: &helper::HelperApp,
    dump_info: &HashMap<String, PackageDumpInfo>,
    admins: &HashSet<String>,
) -> ScannedPackage {
    ScannedPackage {
        package_name: pkg.package_name.clone(),
        label: label_from_helper(&pkg.package_name, app.label.clone()),
        author: app.author.clone().filter(|org| !is_generic_org(org)),
        icon_url: app
            .icon_png
            .as_deref()
            .filter(|bytes| is_image_bytes(bytes))
            .map(icon_bytes_to_data_url),
        is_system: pkg.is_system,
        installer: dump_info
            .get(&pkg.package_name)
            .and_then(|info| info.installer.clone()),
        is_device_admin: admins.contains(&pkg.package_name),
        sideloaded: false,
        installed_recently: false,
        reads_screen: false,
        draws_over: false,
        no_launcher_icon: false,
    }
}

fn label_from_helper(package_name: &str, raw: Option<String>) -> Option<String> {
    match raw.and_then(|value| normalize_label(&value)) {
        Some(label) if label != package_name => Some(label),
        _ => infer_label_from_package(package_name),
    }
}

fn missing_packages<'a>(
    packages: &'a [PackageInfo],
    seen: &HashSet<String>,
) -> Vec<&'a PackageInfo> {
    packages
        .iter()
        .filter(|pkg| !seen.contains(&pkg.package_name))
        .collect()
}

fn resolve_apk_label(meta: &AppMeta, arsc: Option<&[u8]>) -> Option<String> {
    match &meta.label {
        Some(LabelValue::Literal(s)) => normalize_label(s).filter(|l| !is_weak_label(l)),
        Some(LabelValue::Reference(id)) => arsc
            .and_then(|data| lookup_string(data, *id))
            .and_then(|s| normalize_label(&s))
            .filter(|s| !is_weak_label(s)),
        None => None,
    }
}

#[cfg(test)]
fn resolve_label(meta: &AppMeta, arsc: Option<&[u8]>, package_name: &str) -> Option<String> {
    resolve_apk_label(meta, arsc)
        .or_else(|| known_label(package_name))
        .or_else(|| infer_label_from_package(package_name))
}

fn resolve_icon(
    bridge: &AdbBridge,
    package_name: &str,
    apk_path: Option<&str>,
    meta: &AppMeta,
    arsc: Option<&[u8]>,
) -> Option<String> {
    let apk_paths = candidate_apk_paths(apk_path);
    let paths = meta
        .icon_res
        .and_then(|id| arsc.map(|data| lookup_all_strings(data, id)))
        .unwrap_or_default();
    let mut paths = paths;
    if pick_icon_path(&paths).is_none() {
        extend_paths_from_xml(
            bridge,
            package_name,
            apk_path,
            &apk_paths,
            arsc,
            &[],
            &mut paths,
        );
    }
    if pick_icon_path(&paths).is_none() {
        let extra_arsc = load_split_arscs(bridge, package_name, &apk_paths);
        if let Some(id) = meta.icon_res {
            for table in &extra_arsc {
                for s in lookup_all_strings(table, id) {
                    if !paths.contains(&s) {
                        paths.push(s);
                    }
                }
            }
        }
        extend_paths_from_xml(
            bridge,
            package_name,
            apk_path,
            &apk_paths,
            arsc,
            &extra_arsc,
            &mut paths,
        );
    }
    let picked = pick_icon_path(&paths);
    let mut url = picked
        .as_ref()
        .and_then(|path| fetch_icon_data_url(bridge, package_name, &apk_paths, path));
    if url.is_none() {
        for fallback in FALLBACK_LAUNCHER_ENTRIES {
            if paths.iter().any(|p| p == fallback) {
                continue;
            }
            if let Some(found) = fetch_icon_data_url(bridge, package_name, &apk_paths, fallback) {
                url = Some(found);
                break;
            }
        }
    }
    url
}

fn load_split_arscs(bridge: &AdbBridge, package_name: &str, apk_paths: &[String]) -> Vec<Vec<u8>> {
    apk_paths
        .iter()
        .skip(1)
        .filter_map(|path| {
            let bytes = fetch_apk_entry(bridge, package_name, Some(path), "resources.arsc")?;
            (bytes.len() > 256).then_some(bytes)
        })
        .collect()
}

fn extend_paths_from_xml(
    bridge: &AdbBridge,
    package_name: &str,
    apk_path: Option<&str>,
    apk_paths: &[String],
    base_arsc: Option<&[u8]>,
    extra_arsc: &[Vec<u8>],
    paths: &mut Vec<String>,
) {
    let mut seen = HashSet::new();
    for _ in 0..4 {
        let xml_paths: Vec<String> = paths
            .iter()
            .filter(|p| p.ends_with(".xml") && seen.insert((*p).clone()))
            .cloned()
            .collect();
        if xml_paths.is_empty() {
            break;
        }
        for xml_path in xml_paths {
            let Some(xml) =
                fetch_apk_entry(bridge, package_name, apk_path, &xml_path).or_else(|| {
                    apk_paths
                        .iter()
                        .find_map(|p| fetch_apk_entry(bridge, package_name, Some(p), &xml_path))
                })
            else {
                continue;
            };
            for resid in xml_drawable_refs(&xml) {
                if let Some(base) = base_arsc {
                    for s in lookup_all_strings(base, resid) {
                        if !paths.contains(&s) {
                            paths.push(s);
                        }
                    }
                }
                for table in extra_arsc {
                    for s in lookup_all_strings(table, resid) {
                        if !paths.contains(&s) {
                            paths.push(s);
                        }
                    }
                }
            }
        }
    }
}

fn xml_drawable_refs(xml: &[u8]) -> Vec<u32> {
    let Some(doc) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_manifest(xml)))
        .ok()
        .and_then(Result::ok)
    else {
        return Vec::new();
    };
    let Some(root) = doc.get_root() else {
        return Vec::new();
    };
    let mut foreground = Vec::new();
    let mut other = Vec::new();
    collect_xml_drawables(root, false, &mut foreground, &mut other);
    if foreground.is_empty() {
        other
    } else {
        foreground
    }
}

fn collect_xml_drawables(
    node: &Node,
    in_foreground: bool,
    foreground: &mut Vec<u32>,
    other: &mut Vec<u32>,
) {
    let Node::Element(el) = node else {
        return;
    };
    let tag = el.get_tag();
    let now_fg = in_foreground || tag == "foreground";
    for (key, value) in el.get_attributes() {
        if key == "android:drawable" || key == "drawable" || key.ends_with(":drawable") {
            if let Some(id) = parse_resource_ref(value) {
                if id >> 24 == 0x7f && !foreground.contains(&id) && !other.contains(&id) {
                    if now_fg {
                        foreground.push(id);
                    } else {
                        other.push(id);
                    }
                }
            }
        }
    }
    for child in el.get_children() {
        collect_xml_drawables(child, now_fg, foreground, other);
    }
}

const FALLBACK_LAUNCHER_ENTRIES: &[&str] = &[
    "res/mipmap-xxhdpi-v4/ic_launcher.png",
    "res/mipmap-xxxhdpi-v4/ic_launcher.png",
    "res/mipmap-xxhdpi-v4/ic_launcher.webp",
    "res/mipmap-xxxhdpi-v4/ic_launcher.webp",
    "res/mipmap-xhdpi-v4/ic_launcher.png",
    "res/mipmap-hdpi-v4/ic_launcher.png",
];

fn fetch_icon_data_url(
    bridge: &AdbBridge,
    package_name: &str,
    apk_paths: &[String],
    entry: &str,
) -> Option<String> {
    if apk_paths.is_empty() {
        let bytes = fetch_apk_entry(bridge, package_name, None, entry)?;
        return is_image_bytes(&bytes).then(|| icon_bytes_to_data_url(&bytes));
    }
    for path in apk_paths {
        if let Some(bytes) = fetch_apk_entry(bridge, package_name, Some(path), entry) {
            if is_image_bytes(&bytes) {
                return Some(icon_bytes_to_data_url(&bytes));
            }
        }
    }
    None
}

fn candidate_apk_paths(apk_path: Option<&str>) -> Vec<String> {
    let Some(base) = apk_path else {
        return Vec::new();
    };
    let mut paths = vec![base.to_string()];
    if let Some((dir, file)) = base.rsplit_once('/') {
        if file == "base.apk" {
            for split in [
                "split_config.xxxhdpi.apk",
                "split_config.xxhdpi.apk",
                "split_config.xhdpi.apk",
            ] {
                paths.push(format!("{dir}/{split}"));
            }
        }
    }
    paths
}

fn pick_icon_path(paths: &[String]) -> Option<String> {
    paths
        .iter()
        .filter(|p| is_apk_image_path(p))
        .max_by_key(|p| icon_density_score(p))
        .cloned()
}

fn icon_density_score(path: &str) -> i32 {
    let p = path.to_ascii_lowercase();
    let mut score = 1;
    if p.contains("xxxhdpi") {
        score += 40;
    } else if p.contains("xxhdpi") {
        score += 30;
    } else if p.contains("xhdpi") {
        score += 20;
    } else if p.contains("hdpi") {
        score += 10;
    }
    if p.ends_with(".png") {
        score += 1;
    }
    if p.contains("1x1") {
        score -= 50;
    }
    score
}

fn normalize_label(raw: &str) -> Option<String> {
    let s = raw
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!s.is_empty()).then_some(s)
}

fn is_apk_image_path(path: &str) -> bool {
    let p = path.trim();
    if !(p.starts_with("res/") || p.starts_with("/res/")) || p.contains("..") {
        return false;
    }
    let name = p.rsplit('/').next().unwrap_or("");
    if name.is_empty() || name.ends_with(".xml") {
        return false;
    }
    name.ends_with(".png") || name.ends_with(".webp") || !name.contains('.')
}

fn parse_application_meta(manifest: &[u8]) -> AppMeta {
    let Ok(Ok(doc)) =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_manifest(manifest)))
    else {
        return AppMeta {
            label: None,
            icon_res: None,
        };
    };
    let Some(root) = doc.get_root().as_ref() else {
        return AppMeta {
            label: None,
            icon_res: None,
        };
    };
    find_application_meta(root).unwrap_or(AppMeta {
        label: None,
        icon_res: None,
    })
}

fn find_application_meta(node: &Node) -> Option<AppMeta> {
    match node {
        Node::Element(el) => {
            if el.get_tag() == "application" {
                let attrs = el.get_attributes();
                let label = attrs
                    .get("android:label")
                    .or_else(|| attrs.get("label"))
                    .and_then(|v| parse_label_value(v));
                let icon_res = attrs
                    .get("android:icon")
                    .or_else(|| attrs.get("icon"))
                    .and_then(|v| parse_resource_ref(v));
                return Some(AppMeta { label, icon_res });
            }
            for child in el.get_children() {
                if let Some(meta) = find_application_meta(child) {
                    return Some(meta);
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_label_value(raw: &str) -> Option<LabelValue> {
    if let Some(id) = parse_resource_ref(raw) {
        return Some(LabelValue::Reference(id));
    }
    let value = raw.trim();
    if value.is_empty() || value.starts_with("ResourceValueType::") || value.starts_with('@') {
        return None;
    }
    Some(LabelValue::Literal(value.to_string()))
}

const RECENT_MS: i64 = 7 * 24 * 60 * 60 * 1000;

struct RiskSignals {
    a11y: HashSet<String>,
    overlay: HashSet<String>,
    launchers: Option<HashSet<String>>,
}

impl RiskSignals {
    fn load(bridge: &AdbBridge) -> Self {
        let a11y = bridge
            .shell("dumpsys accessibility")
            .map(|text| parse_accessibility_packages(&text))
            .unwrap_or_default();
        let overlay = bridge
            .shell("appops query-op SYSTEM_ALERT_WINDOW allow")
            .map(|text| parse_appops_packages(&text))
            .unwrap_or_default();
        let launchers = bridge
            .shell("cmd package query-activities -a android.intent.action.MAIN -c android.intent.category.LAUNCHER")
            .ok()
            .map(|text| parse_component_packages(&text))
            .filter(|set| !set.is_empty());
        Self {
            a11y,
            overlay,
            launchers,
        }
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn attach_risk(
    pkg: &mut ScannedPackage,
    info: Option<&PackageDumpInfo>,
    signals: &RiskSignals,
    now_ms: i64,
) {
    pkg.sideloaded = is_sideloaded(pkg.installer.as_deref(), pkg.is_system);
    pkg.installed_recently = !pkg.is_system
        && info
            .and_then(|info| info.first_install_ms)
            .is_some_and(|ms| ms <= now_ms && now_ms.saturating_sub(ms) < RECENT_MS);
    pkg.reads_screen = signals.a11y.contains(&pkg.package_name);
    pkg.draws_over = signals.overlay.contains(&pkg.package_name);
    pkg.no_launcher_icon = !pkg.is_system
        && signals
            .launchers
            .as_ref()
            .is_some_and(|set| !set.contains(&pkg.package_name));
}

fn is_sideloaded(installer: Option<&str>, is_system: bool) -> bool {
    if is_system {
        return false;
    }
    matches!(installer, Some(name) if !name.is_empty() && name != "com.android.vending")
}

fn parse_install_time(line: &str) -> Option<i64> {
    let rest = line.trim().strip_prefix("firstInstallTime=")?.trim();
    if rest.is_empty() || rest == "0" {
        return None;
    }
    if let Ok(n) = rest.parse::<i64>() {
        if n <= 0 {
            return None;
        }
        return Some(if n < 10_000_000_000 { n * 1000 } else { n });
    }
    chrono::NaiveDateTime::parse_from_str(rest, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| dt.and_utc().timestamp_millis())
}

fn parse_accessibility_packages(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut capture = false;
    for line in text.lines() {
        if line.to_ascii_lowercase().contains("enabled services") {
            capture = true;
            collect_component_packages(line, &mut out);
            continue;
        }
        if !capture {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') || line.trim_start().starts_with('{') {
            collect_component_packages(line, &mut out);
        } else if !line.trim().is_empty() {
            capture = false;
        }
    }
    out
}

fn parse_appops_packages(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in text.lines() {
        let Some((head, _)) = line.split_once(": allow") else {
            continue;
        };
        let Some(pkg) = head.split_whitespace().last() else {
            continue;
        };
        if pkg.contains('.') {
            out.insert(pkg.to_string());
        }
    }
    out
}

pub fn parse_component_packages(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_component_packages(text, &mut out);
    out
}

fn collect_component_packages(text: &str, out: &mut HashSet<String>) {
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '{' | '}' | ',' | ';' | '[' | ']' | '"' | '\'')
    }) {
        let Some((pkg, _)) = token.split_once('/') else {
            continue;
        };
        if pkg.contains('.') && !pkg.contains(':') {
            out.insert(pkg.to_string());
        }
    }
}

fn parse_bulk_dumpsys(output: &str) -> HashMap<String, PackageDumpInfo> {
    let mut map = HashMap::new();
    let mut current_pkg: Option<String> = None;

    for line in output.lines() {
        if let Some(pkg) = parse_package_header(line) {
            current_pkg = Some(pkg);
            continue;
        }
        if let Some(ref pkg) = current_pkg {
            let entry: &mut PackageDumpInfo = map.entry(pkg.clone()).or_default();
            if entry.installer.is_none() {
                if let Some(inst) = extract_installer_from_line(line) {
                    entry.installer = Some(inst);
                }
            }
            if entry.first_install_ms.is_none() {
                if let Some(ms) = parse_install_time(line) {
                    entry.first_install_ms = Some(ms);
                }
            }
            if line.contains("certificate DN:") || line.contains("Signer #") {
                if let Some(org) = extract_signing_from_line(line) {
                    if !is_generic_org(&org) {
                        entry.signing_org = Some(org);
                    }
                }
            }
        }
    }

    map
}

fn parse_package_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("Package [") {
        return None;
    }
    let start = trimmed.find('[')? + 1;
    let end = trimmed[start..].find(']')? + start;
    let pkg = trimmed[start..end].trim();
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}

fn extract_installer_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("installerPackageName=") {
        return None;
    }
    trimmed
        .split('=')
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "null")
}

fn extract_signing_from_line(line: &str) -> Option<String> {
    extract_dn_field(line, "O=")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Shell snippet that leaves the APK path in `$APK`: uses the known path when
/// available, otherwise resolves it on-device via `pm path`.
fn apk_path_prelude(package_name: &str, apk_path: Option<&str>) -> String {
    match apk_path {
        Some(p) => format!("APK={}", shell_quote(p)),
        None => format!(
            "APK=$(pm path {} 2>/dev/null | head -1 | cut -d: -f2 | tr -d '\\r')",
            shell_quote(package_name)
        ),
    }
}

/// `unzip -p` command. Paths from the APK are single-quoted so `$(...)` stays text.
fn apk_unzip_command(package_name: &str, apk_path: Option<&str>, entry: &str) -> Option<String> {
    if entry.is_empty() || entry.contains("..") || entry.contains('\0') {
        return None;
    }
    Some(format!(
        "{}; [ -n \"$APK\" ] && unzip -p \"$APK\" {}",
        apk_path_prelude(package_name, apk_path),
        shell_quote(entry)
    ))
}

fn fetch_apk_entry(
    bridge: &AdbBridge,
    package_name: &str,
    apk_path: Option<&str>,
    entry: &str,
) -> Option<Vec<u8>> {
    let cmd = apk_unzip_command(package_name, apk_path, entry)?;
    let bytes = if entry == "resources.arsc" {
        exec_out_gz(bridge, &cmd)?
    } else {
        bridge.exec_out(&cmd).ok()?
    };
    is_plausible_apk_entry(&bytes).then_some(bytes)
}

/// Large outputs (resources.arsc can be >100 MB) are USB-bound, so they are
/// gzipped on-device; devices without gzip fall back to the raw stream.
fn exec_out_gz(bridge: &AdbBridge, cmd: &str) -> Option<Vec<u8>> {
    let bytes = bridge
        .exec_out(&format!(
            "({cmd}) 2>/dev/null | (gzip -1 2>/dev/null || cat)"
        ))
        .ok()?;
    maybe_gunzip(bytes)
}

fn maybe_gunzip(bytes: Vec<u8>) -> Option<Vec<u8>> {
    use std::io::Read;
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Some(bytes);
    }
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

fn is_plausible_apk_entry(bytes: &[u8]) -> bool {
    !bytes.is_empty() && !bytes.starts_with(b"unzip:")
}

fn is_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG")
        || (bytes.starts_with(b"RIFF") && bytes.len() > 12 && bytes[8..12] == *b"WEBP")
}

fn icon_bytes_to_data_url(bytes: &[u8]) -> String {
    let mime = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/webp"
    };
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

fn is_weak_label(label: &str) -> bool {
    let trimmed = label.trim();
    if trimmed.contains('.') || trimmed.contains(':') || trimmed.contains('/') {
        return true;
    }
    matches!(
        trimmed,
        "Google" | "Android" | "google" | "android" | "Theme"
    )
}

fn known_label(package_name: &str) -> Option<String> {
    match package_name {
        "com.google.android.gm" => Some("Gmail".into()),
        _ => None,
    }
}

fn infer_label_from_package(package_name: &str) -> Option<String> {
    if let Some(label) = known_label(package_name) {
        return Some(label);
    }
    const SKIP: &[&str] = &[
        "com", "org", "net", "android", "app", "mobile", "client", "ipn", "mshop", "shopping",
    ];
    for segment in package_name.split('.').rev() {
        if SKIP.contains(&segment.to_lowercase().as_str()) || segment.len() < 3 {
            continue;
        }
        return Some(title_case_segment(segment));
    }
    None
}

fn title_case_segment(segment: &str) -> String {
    if segment.eq_ignore_ascii_case("chatgpt") {
        return "ChatGPT".into();
    }
    let mut chars = segment.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
fn extract_signing_organization(text: &str) -> Option<String> {
    for line in text.lines() {
        if line.contains("certificate DN:") || line.contains("Signer #") {
            if let Some(org) = extract_dn_field(line, "O=") {
                if !is_generic_org(&org) {
                    return Some(org);
                }
            }
        }
    }
    None
}

fn extract_dn_field(line: &str, key: &str) -> Option<String> {
    let idx = line.find(key)?;
    let rest = &line[idx + key.len()..];
    let end = rest.find(',').unwrap_or(rest.len());
    let value = rest[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_generic_org(org: &str) -> bool {
    let compact: String = org
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if compact == "google" || compact == "googleinc" {
        return true;
    }
    matches!(
        org,
        "Android" | "Android Debug" | "unknown" | "Unknown" | "CN"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_label_keeps_the_device_name() {
        assert_eq!(
            label_from_helper("com.whatsapp", Some("WhatsApp".into())).as_deref(),
            Some("WhatsApp")
        );
        assert_eq!(
            label_from_helper(
                "com.google.android.gm",
                Some("com.google.android.gm".into())
            )
            .as_deref(),
            Some("Gmail")
        );
        assert_eq!(
            label_from_helper("com.whatsapp", None).as_deref(),
            Some("Whatsapp")
        );
    }

    #[test]
    fn missing_packages_are_the_ones_the_helper_skipped() {
        let packages = vec![
            PackageInfo {
                package_name: "com.a".into(),
                is_system: false,
                apk_path: None,
            },
            PackageInfo {
                package_name: "com.b".into(),
                is_system: false,
                apk_path: None,
            },
        ];
        let seen = HashSet::from(["com.a".into()]);
        let missing: Vec<_> = missing_packages(&packages, &seen)
            .iter()
            .map(|pkg| pkg.package_name.as_str())
            .collect();
        assert_eq!(missing, vec!["com.b"]);
    }

    #[test]
    fn helper_author_drops_generic_orgs() {
        let pkg = PackageInfo {
            package_name: "com.whatsapp".into(),
            is_system: false,
            apk_path: None,
        };
        let app = helper::HelperApp {
            package_name: pkg.package_name.clone(),
            label: Some("WhatsApp".into()),
            author: Some("Google Inc.".into()),
            icon_png: None,
        };
        let scanned = package_from_helper(&pkg, &app, &HashMap::new(), &HashSet::new());
        assert!(scanned.author.is_none());
        assert_eq!(scanned.label.as_deref(), Some("WhatsApp"));
        assert!(is_generic_org("Google, Inc"));
        assert!(!is_generic_org("WhatsApp Inc."));
        assert!(!is_generic_org("Facebook Inc."));
    }

    #[test]
    fn parses_signing_organization() {
        let dumpsys = "    Signer #1 certificate DN: CN=App, O=Meta Platforms Inc., C=US";
        assert_eq!(
            extract_signing_organization(dumpsys).as_deref(),
            Some("Meta Platforms Inc.")
        );
    }

    #[test]
    fn parses_bulk_dumpsys_sections() {
        let output = r#"
Package [com.example.app] (abc):
    installerPackageName=com.android.vending
    Signer #1 certificate DN: CN=App, O=Example Corp, C=US
Package [com.other.app] (def):
    installerPackageName=null
"#;
        let map = parse_bulk_dumpsys(output);
        assert_eq!(
            map.get("com.example.app")
                .and_then(|i| i.installer.as_deref()),
            Some("com.android.vending")
        );
        assert_eq!(
            map.get("com.example.app")
                .and_then(|i| i.signing_org.as_deref()),
            Some("Example Corp")
        );
    }

    #[test]
    fn parse_label_value_treats_resource_as_reference() {
        match parse_label_value("ResourceValueType::Reference/2131755036") {
            Some(LabelValue::Reference(id)) => assert_eq!(id, 0x7f10001c),
            _ => panic!("expected resource reference"),
        }
    }

    #[test]
    fn parse_label_value_keeps_literal() {
        match parse_label_value("ChatGPT") {
            Some(LabelValue::Literal(s)) => assert_eq!(s, "ChatGPT"),
            _ => panic!("expected literal"),
        }
        assert!(parse_label_value("ResourceValueType::String").is_none());
    }

    #[test]
    fn resolve_label_uses_manifest_literal() {
        let meta = AppMeta {
            label: Some(LabelValue::Literal("HTML Viewer".into())),
            icon_res: None,
        };
        assert_eq!(
            resolve_label(&meta, None, "com.android.htmlviewer").as_deref(),
            Some("HTML Viewer")
        );
    }

    #[test]
    fn resolve_label_skips_weak_literal() {
        let meta = AppMeta {
            label: Some(LabelValue::Literal("Google".into())),
            icon_res: None,
        };
        assert_eq!(
            resolve_label(&meta, None, "com.google.android.gm").as_deref(),
            Some("Gmail")
        );
    }

    #[test]
    fn is_apk_image_path_accepts_png_under_res() {
        assert!(is_apk_image_path("res/mipmap-xxhdpi/ic_launcher.png"));
        assert!(is_apk_image_path("res/raw/btD"));
        assert!(!is_apk_image_path("res/raw/btt.xml"));
        assert!(!is_apk_image_path("res/mipmap-anydpi-v26/ic_launcher.xml"));
        assert!(!is_apk_image_path("whatsapp_icon.png"));
        assert!(!is_apk_image_path("res/../ic.png"));
    }

    #[test]
    fn known_label_names_gmail() {
        assert_eq!(
            known_label("com.google.android.gm").as_deref(),
            Some("Gmail")
        );
        assert!(known_label("com.google.android.gms").is_none());
    }

    #[test]
    fn infer_label_does_not_turn_gmail_into_google() {
        assert_eq!(
            infer_label_from_package("com.google.android.gm").as_deref(),
            Some("Gmail")
        );
    }

    #[test]
    fn rejects_weak_arsc_labels() {
        assert!(is_weak_label("Theme.AppCompat"));
        assert!(is_weak_label("google.com:youtube-android"));
        assert!(is_weak_label("Google"));
        assert!(is_weak_label("Android"));
        assert!(!is_weak_label("YouTube Music"));
        assert!(!is_weak_label("Gmail"));
    }

    #[test]
    fn does_not_use_google_play_as_author() {
        let dumpsys = "installerPackageName=com.android.vending";
        assert!(extract_signing_organization(dumpsys).is_none());
    }

    #[test]
    fn normalize_label_replaces_nbsp() {
        assert_eq!(
            normalize_label("WhatsApp\u{a0}Business").as_deref(),
            Some("WhatsApp Business")
        );
    }

    #[test]
    fn pick_icon_path_prefers_png_over_xml() {
        let paths = [
            "res/mipmap-anydpi-v26/ic_launcher.xml".into(),
            "res/mipmap-xxhdpi-v4/ic_launcher.png".into(),
            "res/lbs.png".into(),
        ];
        assert_eq!(
            pick_icon_path(&paths).as_deref(),
            Some("res/mipmap-xxhdpi-v4/ic_launcher.png")
        );
    }

    #[test]
    fn pick_icon_path_skips_xml_when_names_are_obfuscated() {
        let paths = [
            "res/fHq.png".into(),
            "res/ilG.png".into(),
            "res/BWP.xml".into(),
        ];
        assert_eq!(pick_icon_path(&paths).as_deref(), Some("res/ilG.png"));
    }

    #[test]
    fn pick_icon_path_accepts_extensionless_webp_in_res_raw() {
        let paths = [
            "res/raw/bto".into(),
            "res/raw/btD".into(),
            "res/raw/btt.xml".into(),
        ];
        assert_eq!(pick_icon_path(&paths).as_deref(), Some("res/raw/btD"));
    }

    #[test]
    fn risk_hints_come_from_dumpsys_not_the_icon_cache() {
        let mut pkg = ScannedPackage {
            package_name: "com.ads.cleaner".into(),
            label: None,
            author: None,
            icon_url: None,
            is_system: false,
            installer: Some("com.android.chrome".into()),
            is_device_admin: false,
            sideloaded: false,
            installed_recently: false,
            reads_screen: false,
            draws_over: false,
            no_launcher_icon: false,
        };
        let info = PackageDumpInfo {
            installer: pkg.installer.clone(),
            signing_org: None,
            first_install_ms: Some(1_000),
        };
        let signals = RiskSignals {
            a11y: HashSet::from(["com.ads.cleaner".into()]),
            overlay: HashSet::from(["com.ads.cleaner".into()]),
            launchers: Some(HashSet::from(["com.real.app".into()])),
        };
        attach_risk(
            &mut pkg,
            Some(&info),
            &signals,
            1_000 + 3 * 24 * 60 * 60 * 1000,
        );
        assert!(pkg.sideloaded);
        assert!(pkg.installed_recently);
        assert!(pkg.reads_screen);
        assert!(pkg.draws_over);
        assert!(pkg.no_launcher_icon);

        pkg.is_system = true;
        pkg.installer = None;
        attach_risk(&mut pkg, Some(&info), &signals, 1_000);
        assert!(!pkg.sideloaded);
        assert!(!pkg.installed_recently);
        assert!(!pkg.no_launcher_icon);
    }

    #[test]
    fn parses_install_time_and_signal_lists() {
        assert_eq!(parse_install_time("firstInstallTime=0"), None);
        assert_eq!(
            parse_install_time("    firstInstallTime=2024-01-02 03:04:05"),
            Some(
                chrono::NaiveDateTime::parse_from_str("2024-01-02 03:04:05", "%Y-%m-%d %H:%M:%S")
                    .unwrap()
                    .and_utc()
                    .timestamp_millis()
            )
        );
        assert_eq!(
            parse_install_time("firstInstallTime=1700000000000"),
            Some(1700000000000)
        );
        let a11y = parse_accessibility_packages(
            "Enabled services:\n    {com.spy/.A11y}\nBinding services:\n",
        );
        assert!(a11y.contains("com.spy"));
        let ops = parse_appops_packages("  com.cover.app: allow; time=0\n");
        assert!(ops.contains("com.cover.app"));
        assert!(parse_component_packages("").is_empty());
    }

    #[test]
    fn apk_unzip_command_quotes_metacharacters() {
        let cmd =
            apk_unzip_command("com.foo", Some("/data/a'b$(id).apk"), "res/$(id)'x.xml").unwrap();
        assert!(cmd.contains("APK='/data/a'\\''b$(id).apk'"), "{cmd}");
        assert!(
            cmd.contains("unzip -p \"$APK\" 'res/$(id)'\\''x.xml'"),
            "{cmd}"
        );
        assert!(apk_unzip_command("com.foo", None, "res/../x.png").is_none());
    }

    #[test]
    fn rejects_unzip_error_as_apk_entry() {
        assert!(!is_plausible_apk_entry(b""));
        assert!(!is_plausible_apk_entry(
            b"unzip: couldn't open /data/app/x/split_config.xxxhdpi.apk: I/O error\n"
        ));
        assert!(is_plausible_apk_entry(&[0x03, 0x00, 0x08, 0x00, 0x10]));
    }

    #[test]
    fn maybe_gunzip_handles_gzip_and_raw() {
        use std::io::Write;
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(&[0x02, 0x00, 0x0c, 0x00]).unwrap();
        let gz = enc.finish().unwrap();
        assert_eq!(maybe_gunzip(gz).unwrap(), vec![0x02, 0x00, 0x0c, 0x00]);
        assert_eq!(maybe_gunzip(b"raw".to_vec()).unwrap(), b"raw".to_vec());
    }

    #[test]
    fn pick_icon_path_skips_1x1_placeholder() {
        let paths = ["res/1x1.png".into(), "res/kp.png".into()];
        assert_eq!(pick_icon_path(&paths).as_deref(), Some("res/kp.png"));
    }

    #[test]
    fn xml_drawable_refs_reads_adaptive_foreground() {
        let xml = include_bytes!("testdata/camera_adaptive.bin");
        assert_eq!(xml_drawable_refs(xml), vec![2131231199]);
    }

    #[test]
    fn candidate_apk_paths_includes_density_splits() {
        let paths = candidate_apk_paths(Some("/data/app/~~x/com.whatsapp.w4b-y/base.apk"));
        assert_eq!(paths[0], "/data/app/~~x/com.whatsapp.w4b-y/base.apk");
        assert!(paths.iter().any(|p| p.ends_with("split_config.xxhdpi.apk")));
    }
}
