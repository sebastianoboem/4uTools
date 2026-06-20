use std::collections::HashMap;

use crate::models::StorageBreakdown;

pub struct DfResult {
    pub total: u64,
    pub free: u64,
}

pub struct StorageSnapshot {
    pub total: u64,
    pub free: u64,
    pub breakdown: StorageBreakdown,
}

struct DiskstatsCategories {
    apps: u64,
    images: u64,
    videos: u64,
    audio: u64,
    downloads: u64,
    other: u64,
}

impl Default for DiskstatsCategories {
    fn default() -> Self {
        Self {
            apps: 0,
            images: 0,
            videos: 0,
            audio: 0,
            downloads: 0,
            other: 0,
        }
    }
}

/// Parse `df -P -k` — prefer emulated storage mount for user-visible free space.
pub fn parse_df_output(output: &str) -> DfResult {
    let mut emulated: Option<(u64, u64)> = None;
    let mut data: Option<(u64, u64)> = None;

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let mount = parts[parts.len() - 1];
        let Ok(blocks) = parts[1].parse::<u64>() else {
            continue;
        };
        let Ok(avail) = parts[3].parse::<u64>() else {
            continue;
        };
        let total = blocks * 1024;
        let free = avail * 1024;

        if mount == "/storage/emulated/0"
            || mount == "/storage/emulated"
            || mount.ends_with("/emulated/0")
            || mount.ends_with("/emulated")
        {
            emulated = Some((total, free));
        } else if mount == "/data" || mount.starts_with("/data/") {
            if data.map(|(t, _)| total > t).unwrap_or(true) {
                data = Some((total, free));
            }
        }
    }

    let (total, free) = emulated.or(data).unwrap_or((0, 0));
    DfResult { total, free }
}

/// `Data-Free: 101363160K / 105515340K total = 96% free`
fn parse_data_free_line(output: &str) -> Option<(u64, u64)> {
    let re =
        regex::Regex::new(r"(?m)Data-Free:\s*(\d+)K\s*/\s*(\d+)K").unwrap();
    let caps = re.captures(output)?;
    let free_kb: u64 = caps[1].parse().ok()?;
    let total_kb: u64 = caps[2].parse().ok()?;
    Some((free_kb, total_kb))
}

fn parse_labeled_bytes(output: &str, label: &str) -> u64 {
    let re = regex::Regex::new(&format!(
        r"(?m)^{}:\s*(\d+)\s*$",
        regex::escape(label)
    ))
    .unwrap();
    re.captures(output)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0)
}

/// Parse labeled `* Size:` lines from dumpsys diskstats (not generic regex on "system").
fn parse_diskstats_categories(output: &str) -> DiskstatsCategories {
    let app_size = parse_labeled_bytes(output, "App Size");
    let app_data = parse_labeled_bytes(output, "App Data Size");
    let app_cache = parse_labeled_bytes(output, "App Cache Size");
    let apps_from_packages = sum_package_storage(output);

    let apps = if app_size > 0 || app_data > 0 || app_cache > 0 {
        app_size + app_data + app_cache
    } else {
        apps_from_packages
    };

    let images = parse_labeled_bytes(output, "Photos Size");
    let videos = parse_labeled_bytes(output, "Videos Size");
    let audio = parse_labeled_bytes(output, "Audio Size");
    let downloads = parse_labeled_bytes(output, "Downloads Size");
    let other = parse_labeled_bytes(output, "Other Size");

    DiskstatsCategories {
        apps,
        images,
        videos,
        audio,
        downloads,
        other,
    }
}

pub fn parse_block_devices(output: &str) -> u64 {
    let mut best = 0u64;
    for line in output.lines() {
        let Some((_, val)) = line.split_once('=') else {
            continue;
        };
        let Ok(sectors) = val.trim().parse::<u64>() else {
            continue;
        };
        let bytes = sectors.saturating_mul(512);
        if bytes > best {
            best = bytes;
        }
    }
    best
}

pub fn parse_storage_props(props: &HashMap<String, String>) -> u64 {
    for key in [
        "ro.boot.storage.size",
        "ro.product.storage.size",
        "persist.sys.storage.size",
    ] {
        if let Some(raw) = props.get(key) {
            if let Some(bytes) = parse_size_string(raw) {
                return bytes;
            }
        }
    }
    0
}

pub fn build_storage_snapshot(
    df_output: &str,
    diskstats_output: &str,
    block_devices_output: &str,
    props: &HashMap<String, String>,
) -> StorageSnapshot {
    let df = parse_df_output(df_output);
    let physical = parse_block_devices(block_devices_output);
    let prop_total = parse_storage_props(props);
    let categories = parse_diskstats_categories(diskstats_output);

    let (total, free, used) = if let Some((free_kb, userdata_kb)) =
        parse_data_free_line(diskstats_output)
    {
        let free_bytes = free_kb.saturating_mul(1024);
        let userdata_bytes = userdata_kb.saturating_mul(1024);
        let marketing = infer_marketing_total(userdata_bytes, physical, prop_total);
        let used_bytes = marketing.saturating_sub(free_bytes);
        (marketing, free_bytes, used_bytes)
    } else {
        let mut total = physical.max(prop_total);
        if total == 0 {
            total = df.total;
        }
        if total > 0 {
            total = infer_marketing_total(total, physical, prop_total);
        }
        let free = df.free;
        let used = total.saturating_sub(free);
        (total, free, used)
    };

    let apps = categories.apps + categories.downloads;
    let photos = categories.images;
    let videos = categories.videos;
    let audio = categories.audio;
    let downloads = categories.downloads;
    let other_labeled = categories.audio + categories.other;
    // Android Settings "System" = residual used (OS, firmware, caches), not the small / partition.
    let system = used.saturating_sub(apps + photos + other_labeled);
    let other = other_labeled;

    let (otg_total, otg_used, otg_free) = parse_otg_volumes(df_output);

    let breakdown = StorageBreakdown {
        apps,
        photos,
        audio,
        videos,
        downloads,
        system,
        other,
        free,
        total,
        otg_total,
        otg_used,
        otg_free,
    };

    StorageSnapshot {
        total,
        free,
        breakdown,
    }
}

/// Userdata partition (~100–110 GB) maps to 128 GB marketing; expand tier matching.
fn infer_marketing_total(userdata_bytes: u64, physical: u64, prop_total: u64) -> u64 {
    let raw = physical.max(prop_total).max(userdata_bytes);
    if raw == 0 {
        return 0;
    }
    const GB: u64 = 1_000_000_000;
    let gb = raw as f64 / GB as f64;
    if gb >= 98.0 && gb <= 118.0 {
        return 128 * GB;
    }
    if gb >= 45.0 && gb <= 58.0 {
        return 64 * GB;
    }
    if gb >= 190.0 && gb <= 250.0 {
        return 256 * GB;
    }
    round_marketing_storage(raw)
}

fn sum_package_storage(output: &str) -> u64 {
    let app_sizes = sum_bracket_array(output, "App Sizes");
    let data_sizes = sum_bracket_array(output, "App Data Sizes");
    let cache_sizes = sum_bracket_array(output, "Cache Sizes");

    if app_sizes.is_empty() {
        return 0;
    }

    let n = app_sizes.len();
    let mut total = 0u64;
    for i in 0..n {
        total += app_sizes.get(i).copied().unwrap_or(0)
            + data_sizes.get(i).copied().unwrap_or(0)
            + cache_sizes.get(i).copied().unwrap_or(0);
    }
    total
}

fn sum_bracket_array(text: &str, label: &str) -> Vec<u64> {
    let mut result = Vec::new();
    for line in text.lines() {
        if !line.contains(label) {
            continue;
        }
        let Some(start) = line.find('[') else {
            continue;
        };
        let Some(end) = line.find(']') else {
            continue;
        };
        let inner = &line[start + 1..end];
        if inner.trim().is_empty() {
            return result;
        }
        for part in inner.split(',') {
            if let Ok(v) = part.trim().parse::<u64>() {
                result.push(v);
            }
        }
        return result;
    }
    result
}

fn parse_size_string(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if let Ok(v) = raw.parse::<u64>() {
        return Some(v);
    }
    let upper = raw.to_uppercase();
    let num: f64 = upper
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .parse()
        .ok()?;
    if upper.contains("TB") {
        Some((num * 1_000_000_000_000.0) as u64)
    } else if upper.contains("GB") {
        Some((num * 1_000_000_000.0) as u64)
    } else if upper.contains("MB") {
        Some((num * 1_000_000.0) as u64)
    } else {
        None
    }
}

fn round_marketing_storage(bytes: u64) -> u64 {
    const GB: u64 = 1_000_000_000;
    let gb = bytes as f64 / GB as f64;
    let tiers = [32.0, 64.0, 128.0, 256.0, 512.0, 1024.0];
    for tier in tiers {
        if gb >= tier * 0.88 && gb <= tier * 1.02 {
            return (tier * GB as f64) as u64;
        }
    }
    bytes
}

pub struct BlockHardwarePartial {
    pub block: String,
    pub vendor: String,
    pub model: String,
    pub serial: String,
}

/// Parse `block=sda vendor=... model=... serial=...` lines from root shell.
pub fn parse_root_block_hardware(output: &str) -> Option<BlockHardwarePartial> {
    for line in output.lines() {
        let line = line.trim();
        if !line.starts_with("block=") {
            continue;
        }
        let mut block = String::new();
        let mut vendor = String::new();
        let mut model = String::new();
        let mut serial = String::new();
        for part in line.split_whitespace() {
            if let Some(v) = part.strip_prefix("block=") {
                block = v.to_string();
            } else if let Some(v) = part.strip_prefix("vendor=") {
                vendor = v.to_string();
            } else if let Some(v) = part.strip_prefix("model=") {
                model = v.to_string();
            } else if let Some(v) = part.strip_prefix("serial=") {
                serial = v.to_string();
            }
        }
        if !vendor.is_empty() || !model.is_empty() || !serial.is_empty() {
            return Some(BlockHardwarePartial {
                block,
                vendor,
                model,
                serial,
            });
        }
    }
    None
}

/// Sum removable volumes (`/mnt/media_rw/*`, portable `/storage/XXXX-XXXX`).
pub fn parse_otg_volumes(df_output: &str) -> (u64, u64, u64) {
    let mut total = 0u64;
    let mut free = 0u64;

    for line in df_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let mount = parts[parts.len() - 1];
        if mount.contains("emulated") || mount == "/data" || mount.starts_with("/data/") {
            continue;
        }
        let is_otg = mount.starts_with("/mnt/media_rw/")
            || (mount.starts_with("/storage/") && mount.contains('-'));
        if !is_otg {
            continue;
        }
        let Ok(blocks) = parts[1].parse::<u64>() else {
            continue;
        };
        let Ok(used_blocks) = parts[2].parse::<u64>() else {
            continue;
        };
        let Ok(avail) = parts[3].parse::<u64>() else {
            continue;
        };
        total += blocks.saturating_mul(1024);
        free += avail.saturating_mul(1024);
        let _ = used_blocks;
    }

    let used = total.saturating_sub(free);
    (total, used, free)
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const GB: f64 = 1_000_000_000.0;
    const MB: f64 = 1_000_000.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else {
        const UNITS: [&str; 3] = ["B", "KB", "MB"];
        let mut value = b;
        let mut i = 0;
        while value >= 1024.0 && i < UNITS.len() - 1 {
            value /= 1024.0;
            i += 1;
        }
        format!("{value:.0} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_prefers_emulated() {
        let out = "Filesystem 1024-blocks Used Available Use% Mounted on\n\
                   /dev/sda1 100000000 20000000 80000000 20% /data\n\
                   /dev/fuse 100000000 20000000 80000000 20% /storage/emulated/0\n";
        let df = parse_df_output(out);
        assert_eq!(df.total, 100_000_000 * 1024);
    }

    #[test]
    fn df_accepts_emulated_without_zero() {
        let out = "Filesystem 1024-blocks Used Available Use% Mounted on\n\
                   /dev/fuse 105515340 4021108 101363160 4% /storage/emulated\n";
        let df = parse_df_output(out);
        assert_eq!(df.total, 105_515_340 * 1024);
        assert_eq!(df.free, 101_363_160 * 1024);
    }

    #[test]
    fn parses_data_free_line() {
        let out = "Data-Free: 101363160K / 105515340K total = 96% free\n";
        let (free_kb, total_kb) = parse_data_free_line(out).unwrap();
        assert_eq!(free_kb, 101_363_160);
        assert_eq!(total_kb, 105_515_340);
    }

    #[test]
    fn marketing_128_from_userdata() {
        let userdata = 105_515_340u64 * 1024;
        assert_eq!(infer_marketing_total(userdata, 0, 0), 128_000_000_000);
    }

    #[test]
    fn parses_app_size_from_diskstats() {
        let out = "System Size: 128000000000\nApp Size: 1000000\n";
        let cat = parse_diskstats_categories(out);
        assert_eq!(cat.apps, 1_000_000);
    }

    #[test]
    fn parses_root_block_line() {
        let out = "block=sda vendor=Samsung model=KLUDG4UHDC serial=1234abc\n";
        let hw = parse_root_block_hardware(out).unwrap();
        assert_eq!(hw.vendor, "Samsung");
        assert_eq!(hw.model, "KLUDG4UHDC");
        assert_eq!(hw.serial, "1234abc");
    }

    #[test]
    fn parses_otg_volume() {
        let out = "Filesystem 1024-blocks Used Available Use% Mounted on\n\
                   /dev/fuse 100000000 20000000 80000000 20% /storage/emulated/0\n\
                   /dev/sdg1 16000000 4000000 12000000 25% /mnt/media_rw/ABCD-1234\n";
        let (total, used, free) = parse_otg_volumes(out);
        assert_eq!(total, 16_000_000 * 1024);
        assert_eq!(free, 12_000_000 * 1024);
        assert_eq!(used, 4_000_000 * 1024);
    }

    #[test]
    fn system_is_residual_used() {
        let diskstats = "Data-Free: 101363160K / 105515340K total = 96% free\n\
            App Size: 2000000000\nApp Data Size: 0\nApp Cache Size: 0\n\
            Photos Size: 5000000\nOther Size: 10000000\n\
            System-Free: 77028K / 1086904K total = 7% free\n";
        let snap = build_storage_snapshot("", diskstats, "", &HashMap::new());
        assert!(snap.breakdown.system > 15_000_000_000);
        assert!(snap.breakdown.system > snap.breakdown.apps + snap.breakdown.photos);
    }
}
