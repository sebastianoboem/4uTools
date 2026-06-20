use std::collections::HashMap;

pub struct HardwarePartial {
    pub screen_resolution: String,
    pub screen_density: String,
    pub wifi_mac: String,
    pub bluetooth_mac: String,
    pub android_id: String,
    pub time_format_24h: String,
    pub cpu_cores: u32,
    pub cpu_max_freq_mhz: u32,
}

pub fn parse_hardware_shell(output: &str, props: &HashMap<String, String>) -> HardwarePartial {
    let mut screen_resolution = String::new();
    let mut screen_density = String::new();
    let mut wifi_mac = String::new();
    let mut bluetooth_mac = String::new();
    let mut android_id = String::new();
    let mut time_format_24h = String::new();
    let mut cpu_cores = 0u32;
    let mut cpu_max_freq_mhz = 0u32;

    let mut section = "";
    for line in output.lines() {
        let line = line.trim();
        if line == "---WM---" {
            section = "wm";
            continue;
        }
        if line == "---SETTINGS---" {
            section = "settings";
            continue;
        }
        if line == "---CPU---" {
            section = "cpu";
            continue;
        }
        if line == "---NET---" {
            section = "net";
            continue;
        }
        match section {
            "wm" => {
                if let Some(rest) = line.strip_prefix("Physical size: ") {
                    screen_resolution = rest.to_string();
                } else if let Some(rest) = line.strip_prefix("Physical density: ") {
                    screen_density = rest.to_string();
                }
            }
            "settings" => {
                if bluetooth_mac.is_empty() && line.len() >= 12 && line.contains(':') {
                    bluetooth_mac = line.to_string();
                } else if android_id.is_empty() && line.chars().all(|c| c.is_ascii_hexdigit()) {
                    android_id = line.to_string();
                } else if time_format_24h.is_empty() && (line == "12" || line == "24" || line == "null") {
                    time_format_24h = match line.as_ref() {
                        "24" => "Yes".to_string(),
                        "12" => "No".to_string(),
                        _ => "Unknown".to_string(),
                    };
                }
            }
            "cpu" => {
                if let Ok(n) = line.parse::<u32>() {
                    if cpu_cores == 0 {
                        cpu_cores = n;
                    } else if cpu_max_freq_mhz == 0 && n > 1000 {
                        cpu_max_freq_mhz = n / 1000;
                    }
                }
            }
            "net" => {
                if wifi_mac.is_empty() && line.contains(':') {
                    wifi_mac = line.to_string();
                }
            }
            _ => {}
        }
    }

    if screen_resolution.is_empty() {
        if let Some(v) = props.get("ro.sf.lcd_density") {
            if !v.is_empty() {
                screen_density = v.clone();
            }
        }
    }

    HardwarePartial {
        screen_resolution,
        screen_density,
        wifi_mac,
        bluetooth_mac,
        android_id,
        time_format_24h,
        cpu_cores,
        cpu_max_freq_mhz,
    }
}

pub fn soc_label(props: &HashMap<String, String>) -> String {
    for key in ["ro.soc.model", "ro.board.platform", "ro.hardware"] {
        if let Some(v) = props.get(key) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    "N/A".to_string()
}

/// Factory/current Wi-Fi MAC from `dumpsys wifi` (`mWifiInfo ... MAC: xx:xx:...`).
pub fn parse_wifi_mac_from_dumpsys(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"MAC:\s*([0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){5})").ok()?;
    re.captures(text).map(|c| c[1].to_string())
}

/// Fingerprint enrollment summary from `dumpsys fingerprint`.
pub fn parse_fingerprint_status(text: &str) -> Option<String> {
    if !text.contains("FingerprintProvider") && !text.contains("fingerprint") {
        return None;
    }
    let re = regex::Regex::new(r#""count"\s*:\s*(\d+)"#).ok()?;
    let mut enrolled = 0u32;
    for caps in re.captures_iter(text) {
        if let Ok(n) = caps[1].parse::<u32>() {
            enrolled = enrolled.max(n);
        }
    }
    Some(if enrolled == 0 {
        "Not enrolled".to_string()
    } else {
        format!("{enrolled} fingerprint(s) enrolled")
    })
}

pub fn cpu_abi(props: &HashMap<String, String>) -> String {
    props
        .get("ro.product.cpu.abilist")
        .or_else(|| props.get("ro.product.cpu.abi"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "N/A".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wifi_mac_from_dumpsys() {
        let out = r#"mWifiInfo SSID: "Test", BSSID: aa:bb, MAC: 52:bc:63:4b:03:40, IP: /192.168.1.1"#;
        assert_eq!(
            parse_wifi_mac_from_dumpsys(out),
            Some("52:bc:63:4b:03:40".to_string())
        );
    }

    #[test]
    fn parses_fingerprint_not_enrolled() {
        let out = r#"FingerprintProvider{"prints":[{"id":0,"count":0,"accept":0}]}"#;
        assert_eq!(
            parse_fingerprint_status(out),
            Some("Not enrolled".to_string())
        );
    }
}
