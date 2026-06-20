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

    BatteryPartial {
        level: if scale > 0 {
            ((level as f64 / scale as f64) * 100.0).round() as u32
        } else {
            level
        },
        health: capacity_health_percent(full_charge_capacity, design_capacity),
        health_code,
        status: status_label(status_code),
        charging_watts,
        design_capacity,
        full_charge_capacity,
        cycle_count: 0,
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

fn capacity_health_percent(max_mah: u32, design_mah: u32) -> u32 {
    if max_mah > 0 && design_mah > 0 {
        ((max_mah as f64 / design_mah as f64) * 100.0).round() as u32
    } else {
        0
    }
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
}
