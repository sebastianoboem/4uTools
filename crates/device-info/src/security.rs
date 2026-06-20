use std::collections::HashMap;

/// Parse IMEI from `service call iphonesubinfo` parcel hex (UTF-16LE digits in 32-bit words).
pub fn parse_parcel_imei(text: &str) -> Option<String> {
    if !text.contains("Parcel") && !text.contains("0x") {
        return None;
    }
    let quoted = digits_from_quoted_ascii(text);
    if quoted.len() >= 15 {
        return Some(quoted.chars().take(15).collect());
    }
    let hex_digits = digits_from_hex_words(text);
    if hex_digits.len() >= 15 {
        Some(hex_digits.chars().take(15).collect())
    } else {
        None
    }
}

fn digits_from_quoted_ascii(text: &str) -> String {
    let re = regex::Regex::new(r"'([^']*)'").unwrap();
    let mut digits = String::new();
    for caps in re.captures_iter(text) {
        for c in caps[1].chars() {
            if c.is_ascii_digit() {
                digits.push(c);
            }
        }
    }
    digits
}

fn digits_from_hex_words(text: &str) -> String {
    let word_re = regex::Regex::new(r"[0-9a-fA-F]{8}").unwrap();
    let mut digits = String::new();
    for m in word_re.find_iter(text) {
        let Ok(w) = u32::from_str_radix(m.as_str(), 16) else {
            continue;
        };
        for chunk in w.to_le_bytes().chunks_exact(2) {
            let ch = u16::from_le_bytes([chunk[0], chunk[1]]);
            if (0x30..=0x39).contains(&ch) {
                digits.push(char::from(ch as u8));
            }
        }
        if digits.len() >= 15 {
            break;
        }
    }
    digits
}

/// Parse IMSI from `service call iphonesubinfo` parcel (same UTF-16LE encoding as IMEI).
pub fn parse_parcel_imsi(text: &str) -> Option<String> {
    parse_parcel_imei(text).and_then(|digits| valid_imsi(&digits))
}

fn valid_imsi(digits: &str) -> Option<String> {
    let len = digits.len();
    if (14..=15).contains(&len) && !digits.starts_with("000000") {
        Some(digits.to_string())
    } else {
        None
    }
}

/// Collect IMSI from telephony dumps, service calls, and getprop.
pub fn extract_imsi(sources: &[&str], props: &HashMap<String, String>) -> String {
    for key in [
        "gsm.sim.imsi",
        "persist.radio.imsi",
        "ril.imsi",
        "gsm.imsi",
    ] {
        if let Some(v) = props.get(key) {
            let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Some(imsi) = valid_imsi(&digits) {
                return imsi;
            }
        }
    }

    for source in sources {
        if let Some(imsi) = parse_parcel_imsi(source) {
            return imsi;
        }

        let patterns = [
            r"(?i)mImsi[=:\s]+(\d{14,15})",
            r"(?i)subscriberId[=:\s]+(\d{14,15})",
            r"(?i)IMSI[^=\n]{0,24}=\s*(\d{14,15})",
        ];
        for pat in patterns {
            let re = regex::Regex::new(pat).unwrap();
            for caps in re.captures_iter(source) {
                if let Some(imsi) = valid_imsi(&caps[1]) {
                    return imsi;
                }
            }
        }
    }

    "N/A".to_string()
}

/// Collect IMEI from multiple ADB outputs and getprop keys.
pub fn extract_imei(sources: &[&str], props: &HashMap<String, String>) -> (String, String) {
    let mut imeis: Vec<String> = Vec::new();

    for key in [
        "gsm.imei",
        "persist.radio.imei",
        "ro.ril.oem.imei",
        "ril.IMEI",
        "ro.gsm.imei",
        "persist.sys.imei",
        "ro.boot.imei",
    ] {
        if let Some(v) = props.get(key) {
            push_valid_imei(&mut imeis, v);
        }
    }

    for source in sources {
        if let Some(imei) = parse_parcel_imei(source) {
            push_valid_imei(&mut imeis, &imei);
        }

        for line in source.lines() {
            let line = line.trim();
            if line.len() == 15 && line.chars().all(|c| c.is_ascii_digit()) {
                push_valid_imei(&mut imeis, line);
            }
        }

        let patterns = [
            r"(?i)mImei[=:\s]+(\d{15})",
            r"(?i)mImei\d[=:\s]+(\d{15})",
            r"(?i)IMEI[^=\n]{0,24}=\s*(\d{15})",
            r"(?i)deviceId[=:\s]+(\d{15})",
        ];
        for pat in patterns {
            let re = regex::Regex::new(pat).unwrap();
            for caps in re.captures_iter(source) {
                push_valid_imei(&mut imeis, &caps[1]);
            }
        }
    }

    (
        imeis
            .first()
            .cloned()
            .unwrap_or_else(|| "N/A (restricted)".to_string()),
        imeis.get(1).cloned().unwrap_or_else(|| "N/A".to_string()),
    )
}

fn push_valid_imei(list: &mut Vec<String>, raw: &str) {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 15 {
        return;
    }
    if digits.starts_with("000000") {
        return;
    }
    if list.iter().any(|i| i == &digits) {
        return;
    }
    list.push(digits);
}

pub fn is_rooted(su_out: &str, magisk_out: &str) -> bool {
    magisk_out.to_lowercase().contains("magisk")
        || su_out.lines().any(|l| l.trim() == "found")
}

pub fn detect_root(su_out: &str, magisk_out: &str) -> String {
    if magisk_out.to_lowercase().contains("magisk") {
        "Yes (Magisk)".to_string()
    } else if su_out.lines().any(|l| l.trim() == "found") {
        "Yes".to_string()
    } else {
        "No".to_string()
    }
}

/// Sysfs battery fields readable only with root.
pub const ROOT_BATTERY_SHELL: &str = r#"
run_root() {
  for su in su /sbin/su /system/bin/su /system/xbin/su; do
    out=$($su -c "$1" 2>/dev/null) && [ -n "$out" ] && printf '%s' "$out" && return 0
  done
  if [ -x /data/adb/magisk/magisk ]; then
    out=$(/data/adb/magisk/magisk su -c "$1" 2>/dev/null) && [ -n "$out" ] && printf '%s' "$out" && return 0
  fi
  return 1
}
for f in charge_full_design charge_full cycle_count manufacturer serial_number; do
  v=$(run_root "cat /sys/class/power_supply/battery/$f" 2>/dev/null) || true
  [ -n "$v" ] && echo "$f=$v"
done
"#;

/// Block device vendor/model/serial via root sysfs.
pub const ROOT_STORAGE_SHELL: &str = r#"
run_root() {
  for su in su /sbin/su /system/bin/su /system/xbin/su; do
    out=$($su -c "$1" 2>/dev/null) && [ -n "$out" ] && printf '%s' "$out" && return 0
  done
  if [ -x /data/adb/magisk/magisk ]; then
    out=$(/data/adb/magisk/magisk su -c "$1" 2>/dev/null) && [ -n "$out" ] && printf '%s' "$out" && return 0
  fi
  return 1
}
disk=$(getprop dev.mnt.rootdisk.odm 2>/dev/null)
[ -z "$disk" ] && disk=sda
for d in $disk mmcblk0 nvme0n1; do
  v=$(run_root "cat /sys/block/$d/device/vendor" 2>/dev/null | tr -d ' \n') || true
  m=$(run_root "cat /sys/block/$d/device/model" 2>/dev/null | tr -d ' \n') || true
  s=$(run_root "cat /sys/block/$d/device/serial" 2>/dev/null | tr -d ' \n') || true
  [ -n "$v$m$s" ] && echo "block=$d vendor=$v model=$m serial=$s"
done
"#;

pub fn detect_frp(props: &HashMap<String, String>) -> String {
    if props.contains_key("ro.frp.pst") || props.contains_key("persist.sys.frp") {
        "Protected".to_string()
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rooted_magisk() {
        assert!(is_rooted("notfound", "package:com.topjohnwu.magisk"));
    }

    #[test]
    fn detects_not_rooted() {
        assert!(!is_rooted("notfound", ""));
    }

    #[test]
    fn parses_cmd_phone_imei() {
        let (a, _) = extract_imei(&["359123456789012\n"], &HashMap::new());
        assert_eq!(a, "359123456789012");
    }

    #[test]
    fn parses_parcel_utf16() {
        let parcel = "Result: Parcel(\n\
            0x00000000: 00000000 0000000f 00360038 00360033 '........8.6.3.6.'\n\
            0x00000010: 00390031 00340030 00330035 00330035 '1.9.0.4.5.3.5.3.'\n\
            0x00000020: 00330031 00000036                   '1.3.6...        '\n)";
        assert_eq!(parse_parcel_imei(parcel), Some("863619045353136".to_string()));
    }
}
