pub struct BatteryPartial {
    pub level: u32,
    pub health: u32,
    pub health_code: u32,
    pub status: String,
    pub charging_watts: String,
    pub design_capacity: u32,
    pub full_charge_capacity: u32,
    pub cycle_count: u32,
    pub temperature_tenths_c: i32,
    pub technology: String,
    pub charge_counter_mah: u32,
    pub voltage_mv: u32,
    pub current_ua: i32,
    pub capacity_level: u32,
    pub capacity_level_label: String,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
}

pub fn parse_dumpsys_battery(output: &str) -> BatteryPartial {
    let get = |key: &str| -> Option<String> {
        let re = regex::Regex::new(&format!(r"(?m)^\s*{key}:\s*(.+)$")).unwrap();
        re.captures(output).map(|c| c[1].trim().to_string())
    };

    let level = get("level").and_then(|s| s.parse().ok()).unwrap_or(0);
    let scale = get("scale").and_then(|s| s.parse().ok()).unwrap_or(100);
    let status_code = get("status").and_then(|s| s.parse().ok()).unwrap_or(0);
    let health_code = get("health").and_then(|s| s.parse().ok()).unwrap_or(0);
    let voltage_mv = get("voltage").and_then(|s| s.parse().ok()).unwrap_or(0);
    let voltage_v = voltage_mv as f64 / 1000.0;
    let current_ua = get("current now").and_then(|s| s.parse().ok()).unwrap_or(0);
    let max_current = get("Max charging current")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let max_voltage = get("Max charging voltage")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let capacity_level = get("Capacity level")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let design_capacity = micro_ah_to_mah(
        get("Design capacity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    let full_charge_capacity = micro_ah_to_mah(
        get("Maximum capacity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    let charge_counter_mah = micro_ah_to_mah(
        get("Charge counter")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    let temperature_tenths_c = get("temperature")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let technology = get("technology").unwrap_or_else(|| "Unknown".to_string());

    let charging_watts = if status_code == 2 && max_current > 0 && max_voltage > 0 {
        charging_watts_from_ua_uv(max_current, max_voltage)
    } else {
        estimate_watts(voltage_v, current_ua)
    };

    // Samsung exposes cumulative discharge units (~1 unit = 1% of one full cycle).
    let samsung_cycles = parse_bracketed_u32(output, "mSavedBatteryUsage").map(|u| u / 100);
    let samsung_asoc = parse_bracketed_u32(output, "mSavedBatteryAsoc")
        .or_else(|| parse_plain_u32(output, "mSavedBatteryBsoh"));

    BatteryPartial {
        level: if scale > 0 {
            ((level as f64 / scale as f64) * 100.0).round() as u32
        } else {
            level
        },
        health: {
            let from_cap = capacity_health_percent(full_charge_capacity, design_capacity);
            if from_cap > 0 {
                from_cap
            } else {
                samsung_asoc.unwrap_or(0)
            }
        },
        health_code,
        status: status_label(status_code),
        charging_watts,
        design_capacity,
        full_charge_capacity,
        cycle_count: samsung_cycles.unwrap_or(0),
        temperature_tenths_c,
        technology,
        charge_counter_mah,
        voltage_mv,
        current_ua,
        capacity_level,
        capacity_level_label: capacity_level_label(capacity_level),
        manufacturer: None,
        serial_number: None,
    }
}

/// Capacities from `dumpsys batterystats` (often available without root).
#[derive(Debug, Default, Clone, Copy)]
pub struct BatterystatsCapacities {
    pub estimated_mah: u32,
    pub learned_mah: u32,
}

pub fn parse_batterystats_capacities(output: &str) -> BatterystatsCapacities {
    let mut out = BatterystatsCapacities::default();
    let re_est = regex::Regex::new(r"(?i)Estimated battery capacity:\s*(\d+)").ok();
    let re_learned = regex::Regex::new(r"(?i)Last learned battery capacity:\s*(\d+)").ok();
    let re_typical = regex::Regex::new(r"(?i)Typical:\s*(\d+)").ok();
    if let Some(re) = &re_est {
        if let Some(c) = re.captures(output) {
            out.estimated_mah = c[1].parse().unwrap_or(0);
        }
    }
    if let Some(re) = &re_learned {
        if let Some(c) = re.captures(output) {
            out.learned_mah = c[1].parse().unwrap_or(0);
        }
    }
    if out.learned_mah == 0 {
        if let Some(re) = &re_typical {
            if let Some(c) = re.captures(output) {
                out.learned_mah = c[1].parse().unwrap_or(0);
            }
        }
    }
    out
}

fn parse_bracketed_u32(output: &str, key: &str) -> Option<u32> {
    let re = regex::Regex::new(&format!(r"(?m)^\s*{key}:\s*\[?(\d+)\]?\s*$")).ok()?;
    re.captures(output)?.get(1)?.as_str().parse().ok()
}

fn parse_plain_u32(output: &str, key: &str) -> Option<u32> {
    let re = regex::Regex::new(&format!(r"(?m)^\s*{key}:\s*(\d+)\s*$")).ok()?;
    re.captures(output)?.get(1)?.as_str().parse().ok()
}

pub fn parse_sysfs_battery(output: &str) -> BatteryPartial {
    let mut files = std::collections::HashMap::new();
    for line in output.lines() {
        if let Some((k, v)) = line.split_once('=') {
            files.insert(k.to_string(), v.trim().to_string());
        }
    }

    let parse_num = |key: &str| files.get(key).and_then(|s| s.parse().ok()).unwrap_or(0u32);

    let design = micro_ah_to_mah(parse_num("charge_full_design"));
    let full = micro_ah_to_mah(parse_num("charge_full"));
    let manufacturer = files.get("manufacturer").cloned().filter(|s| !s.is_empty());
    let serial_number = files.get("serial_number").cloned().filter(|s| !s.is_empty());

    BatteryPartial {
        cycle_count: parse_num("cycle_count"),
        design_capacity: design,
        full_charge_capacity: full,
        level: 0,
        health: capacity_health_percent(full, design),
        health_code: 0,
        status: String::new(),
        charging_watts: String::new(),
        temperature_tenths_c: 0,
        technology: String::new(),
        charge_counter_mah: 0,
        voltage_mv: 0,
        current_ua: 0,
        capacity_level: 3,
        capacity_level_label: "Normal".to_string(),
        manufacturer,
        serial_number,
    }
}

pub fn merged_health(dumpsys: &BatteryPartial, sysfs: &BatteryPartial) -> u32 {
    if dumpsys.full_charge_capacity > 0 && dumpsys.design_capacity > 0 {
        capacity_health_percent(dumpsys.full_charge_capacity, dumpsys.design_capacity)
    } else if sysfs.full_charge_capacity > 0 && sysfs.design_capacity > 0 {
        capacity_health_percent(sysfs.full_charge_capacity, sysfs.design_capacity)
    } else if dumpsys.health > 0 {
        dumpsys.health
    } else {
        0
    }
}

pub fn merge_battery_sysfs(dumpsys: &mut BatteryPartial, sysfs: &BatteryPartial) {
    if dumpsys.manufacturer.is_none() {
        dumpsys.manufacturer = sysfs.manufacturer.clone();
    }
    if dumpsys.serial_number.is_none() {
        dumpsys.serial_number = sysfs.serial_number.clone();
    }
    if dumpsys.cycle_count == 0 {
        dumpsys.cycle_count = sysfs.cycle_count;
    }
}

pub fn format_temperature(tenths_c: i32) -> String {
    if tenths_c == 0 {
        return "N/A".to_string();
    }
    format!("{:.2} °C", tenths_c as f64 / 10.0)
}

fn micro_ah_to_mah(micro_ah: u32) -> u32 {
    if micro_ah > 10_000 {
        micro_ah / 1000
    } else {
        micro_ah
    }
}

pub fn capacity_health_percent(max_mah: u32, design_mah: u32) -> u32 {
    if max_mah > 0 && design_mah > 0 {
        ((max_mah as f64 / design_mah as f64) * 100.0).round() as u32
    } else {
        0
    }
}

/// Reject nonsense full-charge readings (e.g. cycle count leaking as 50 mAh).
pub fn is_plausible_full_capacity(full_mah: u32, design_mah: u32, charge_counter_mah: u32) -> bool {
    if full_mah < 500 {
        return false;
    }
    if charge_counter_mah > 0 && full_mah < charge_counter_mah {
        return false;
    }
    if design_mah > 0 {
        if full_mah > design_mah.saturating_mul(2) {
            return false;
        }
        if full_mah * 5 < design_mah {
            return false;
        }
    }
    true
}

/// Estimate full-charge mAh from remaining charge counter and battery %.
pub fn estimate_full_from_charge_counter(charge_counter_mah: u32, level_percent: u32) -> Option<u32> {
    if charge_counter_mah == 0 || !(5..=100).contains(&level_percent) {
        return None;
    }
    let est = ((charge_counter_mah as f64) * 100.0 / level_percent as f64).round() as u32;
    Some(est)
}

/// True when a "full" reading is basically the design value (OEM nominal), not wear.
pub fn looks_like_design_capacity(full_mah: u32, design_mah: u32) -> bool {
    design_mah > 0 && full_mah + 50 >= design_mah
}

/// Prefer charge-counter estimate when a claimed full capacity looks too optimistic.
pub fn refine_full_capacity(
    claimed_full: u32,
    design_mah: u32,
    charge_counter_mah: u32,
    level_percent: u32,
) -> u32 {
    let Some(est) = estimate_full_from_charge_counter(charge_counter_mah, level_percent) else {
        return claimed_full;
    };
    if !is_plausible_full_capacity(est, design_mah, charge_counter_mah) {
        return claimed_full;
    }
    // High SOC: charge counter ≈ present full capacity.
    if level_percent >= 85 && claimed_full > 0 && est + 100 < claimed_full {
        return est;
    }
    // Claimed full equals design, but counter shows wear.
    if looks_like_design_capacity(claimed_full, design_mah)
        && est + 100 < design_mah
        && level_percent >= 50
    {
        return est;
    }
    if claimed_full == 0 {
        return est;
    }
    claimed_full
}

fn capacity_level_label(level: u32) -> String {
    match level {
        1 => "Critical".to_string(),
        2 => "Low".to_string(),
        3 => "Normal".to_string(),
        4 => "High".to_string(),
        5 => "Full".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn status_label(code: u32) -> String {
    match code {
        2 => "Charging",
        3 => "Discharging",
        4 => "Not Charging",
        5 => "Full",
        _ => "Unknown",
    }
    .to_string()
}

fn charging_watts_from_ua_uv(current_ua: i32, voltage_uv: i32) -> String {
    if current_ua == 0 || voltage_uv == 0 {
        return "0W".to_string();
    }
    let watts = (voltage_uv as f64 / 1_000_000.0) * (current_ua as f64 / 1_000_000.0);
    format_charging_watts(watts)
}

fn estimate_watts(voltage: f64, current_ua: i32) -> String {
    if voltage == 0.0 || current_ua == 0 {
        return "Not charging".to_string();
    }
    let watts = (voltage * current_ua as f64).abs() / 1_000_000.0;
    format_charging_watts(watts)
}

fn format_charging_watts(watts: f64) -> String {
    if watts < 0.1 {
        "Not charging".to_string()
    } else if watts < 7.5 {
        format!("Slow Charging {watts:.1}W")
    } else if watts < 15.0 {
        format!("Charging {watts:.1}W")
    } else {
        format!("Fast Charging {watts:.1}W")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_maximum_and_design_capacity() {
        let out = "  level: 85\n  scale: 100\n  status: 2\n  health: 2\n  voltage: 4332\n  Maximum capacity: 4360000\n  Design capacity: 4455000\n  temperature: 293\n  technology: Li-ion\n  Max charging current: 500000\n  Max charging voltage: 5000000\n  Capacity level: 3\n";
        let b = parse_dumpsys_battery(out);
        assert_eq!(b.full_charge_capacity, 4360);
        assert_eq!(b.design_capacity, 4455);
        assert_eq!(b.health, 98);
        assert_eq!(b.voltage_mv, 4332);
    }

    #[test]
    fn rejects_cycle_count_as_full_capacity() {
        assert!(!is_plausible_full_capacity(50, 4510, 1948));
        assert!(is_plausible_full_capacity(4058, 4510, 1948));
    }

    #[test]
    fn estimates_full_from_charge_counter() {
        assert_eq!(estimate_full_from_charge_counter(1948, 48), Some(4058));
        assert_eq!(estimate_full_from_charge_counter(1948, 0), None);
    }

    #[test]
    fn refines_oneplus_estimated_design_disguised_as_full() {
        // Estimated 4510 == design; charge counter at 100% shows real full ~3703.
        let refined = refine_full_capacity(4510, 4510, 3703, 100);
        assert_eq!(refined, 3703);
        assert_eq!(capacity_health_percent(refined, 4510), 82);
    }

    #[test]
    fn parses_samsung_usage_cycles_and_asoc() {
        let out = r#"
  level: 91
  scale: 100
  status: 2
  health: 2
  Charge counter: 3080000
  technology: Li-ion
  mSavedBatteryAsoc: [86]
  mSavedBatteryUsage: [127096]
  mSavedBatteryBsoh: 90
"#;
        let b = parse_dumpsys_battery(out);
        assert_eq!(b.cycle_count, 1270);
        assert_eq!(b.health, 86);
        assert_eq!(b.charge_counter_mah, 3080);
    }

    #[test]
    fn parses_batterystats_learned_capacity() {
        let out = r#"
  Estimated battery capacity: 3373 mAh
  Last learned battery capacity: 3700 mAh
  Min learned battery capacity: 3700 mAh
"#;
        let c = parse_batterystats_capacities(out);
        assert_eq!(c.estimated_mah, 3373);
        assert_eq!(c.learned_mah, 3700);
    }
}
