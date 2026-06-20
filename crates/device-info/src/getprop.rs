use std::collections::HashMap;

pub fn parse_getprop(output: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    let re = regex::Regex::new(r"^\[([^\]]+)\]: \[([^\]]*)\]$").unwrap();
    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            props.insert(caps[1].to_string(), caps[2].to_string());
        }
    }
    props
}

pub fn get_prop(props: &HashMap<String, String>, key: &str) -> String {
    prop_optional(props, key).unwrap_or_else(|| "N/A".to_string())
}

fn prop_optional(props: &HashMap<String, String>, key: &str) -> Option<String> {
    props
        .get(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Sales region with OEM, SKU, locale and carrier fallbacks.
pub fn resolve_sales_region(props: &HashMap<String, String>) -> String {
    for key in [
        "ro.product.locale.region",
        "ro.vendor.oplus.regionmark",
        "ro.boot.regionmark",
        "ro.oplus.regionmark",
    ] {
        if let Some(v) = prop_optional(props, key) {
            return v;
        }
    }

    if let Some(model) = prop_optional(props, "ro.product.model") {
        if let Some(region) = sales_region_from_model(&model) {
            return region;
        }
    }

    for key in ["persist.sys.locale", "ro.product.locale"] {
        if let Some(locale) = prop_optional(props, key) {
            if let Some(country) = country_from_locale(&locale) {
                return country;
            }
        }
    }

    for key in [
        "gsm.sim.operator.iso-country",
        "gsm.operator.iso-country",
    ] {
        if let Some(raw) = prop_optional(props, key) {
            let code = raw.split(',').next().unwrap_or("").trim();
            if !code.is_empty() {
                return country_label_from_code(code);
            }
        }
    }

    "N/A".to_string()
}

/// Manufacturing / factory date with build-date fallbacks when OEM fields are absent.
pub fn resolve_manufacturing_date(props: &HashMap<String, String>) -> String {
    for key in [
        "ro.bootimage.build.date",
        "ro.vendor.build.date",
        "ro.product.build.date",
        "ro.build.date",
        "ro.odm.build.date",
        "ro.system.build.date",
    ] {
        if let Some(v) = prop_optional(props, key) {
            return simplify_build_date(&v);
        }
    }

    for key in [
        "ro.bootimage.build.date.utc",
        "ro.build.date.utc",
        "ro.product.build.date.utc",
    ] {
        if let Some(v) = prop_optional(props, key) {
            if let Ok(ts) = v.parse::<i64>() {
                return format_unix_date(ts);
            }
        }
    }

    "N/A".to_string()
}

fn country_from_locale(locale: &str) -> Option<String> {
    let locale = locale.trim();
    let country = locale
        .split_once('-')
        .or_else(|| locale.split_once('_'))
        .map(|(_, c)| c)?;
    if country.len() == 2 && country.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(country_label_from_code(country))
    } else {
        None
    }
}

fn country_label_from_code(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "it" => "Italy".to_string(),
        "us" => "United States".to_string(),
        "gb" | "uk" => "United Kingdom".to_string(),
        "de" => "Germany".to_string(),
        "fr" => "France".to_string(),
        "es" => "Spain".to_string(),
        "cn" => "China".to_string(),
        "in" => "India".to_string(),
        "jp" => "Japan".to_string(),
        "au" => "Australia".to_string(),
        "ca" => "Canada".to_string(),
        _ => code.to_uppercase(),
    }
}

fn sales_region_from_model(model: &str) -> Option<String> {
    match model.trim().to_uppercase().as_str() {
        // OnePlus 8 Pro
        "IN2020" => Some("China".to_string()),
        "IN2021" => Some("India".to_string()),
        "IN2023" => Some("Europe".to_string()),
        "IN2025" => Some("United States".to_string()),
        // OnePlus 8
        "IN2010" => Some("China".to_string()),
        "IN2011" => Some("India".to_string()),
        "IN2013" => Some("Europe".to_string()),
        "IN2015" => Some("United States".to_string()),
        // OnePlus 8T
        "KB2000" => Some("China".to_string()),
        "KB2001" => Some("India".to_string()),
        "KB2003" => Some("Europe".to_string()),
        "KB2005" => Some("United States".to_string()),
        _ => None,
    }
}

fn simplify_build_date(raw: &str) -> String {
    // "Fri May 29 08:09:58 UTC 2026" -> "2026-05-29"
    let re = regex::Regex::new(
        r"^[A-Za-z]{3}\s+([A-Za-z]{3})\s+(\d{1,2})\s+\d{2}:\d{2}:\d{2}\s+\S+\s+(\d{4})$",
    )
    .unwrap();
    if let Some(caps) = re.captures(raw.trim()) {
        let month = month_number(&caps[1]);
        let day: u32 = caps[2].parse().unwrap_or(1);
        let year: u32 = caps[3].parse().unwrap_or(0);
        if month > 0 && year > 0 {
            return format!("{year:04}-{month:02}-{day:02}");
        }
    }
    raw.trim().to_string()
}

fn month_number(mon: &str) -> u32 {
    match mon.to_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => 0,
    }
}

fn format_unix_date(ts: i64) -> String {
    // Enough for display without adding chrono dependency.
    let days = ts / 86_400;
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u32;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    let d = (remaining + 1).max(1);
    format!("{y:04}-{m:02}-{d:02}")
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn format_bootloader(state: &str) -> String {
    match state.to_lowercase().as_str() {
        "green" => "Locked".to_string(),
        "orange" => "Unlocked".to_string(),
        "yellow" => "Unlocked (Custom)".to_string(),
        _ => state.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_region_from_model() {
        let mut props = HashMap::new();
        props.insert("ro.product.model".to_string(), "IN2023".to_string());
        assert_eq!(resolve_sales_region(&props), "Europe");
    }

    #[test]
    fn resolves_region_from_locale() {
        let mut props = HashMap::new();
        props.insert("persist.sys.locale".to_string(), "it-IT".to_string());
        assert_eq!(resolve_sales_region(&props), "Italy");
    }

    #[test]
    fn resolves_manufacturing_from_build_date() {
        let mut props = HashMap::new();
        props.insert(
            "ro.build.date".to_string(),
            "Fri May 29 08:09:58 UTC 2026".to_string(),
        );
        assert_eq!(resolve_manufacturing_date(&props), "2026-05-29");
    }
}
