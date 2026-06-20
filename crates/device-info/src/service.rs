use adb_bridge::AdbBridge;

use crate::battery::{
    format_temperature, merge_battery_sysfs, merged_health, parse_dumpsys_battery,
    parse_sysfs_battery,
};
use crate::details::{
    build_battery_details, build_device_details, build_storage_details, build_verification_checks,
    DetailsInput,
};
use crate::getprop::{format_bootloader, get_prop, parse_getprop, resolve_manufacturing_date, resolve_sales_region};
use crate::hardware::{parse_fingerprint_status, parse_hardware_shell, parse_wifi_mac_from_dumpsys};
use crate::models::DeviceSummary;
use crate::security::{
    detect_frp, detect_root, extract_imei, extract_imsi, is_rooted, ROOT_BATTERY_SHELL,
    ROOT_STORAGE_SHELL,
};
use crate::storage::{build_storage_snapshot, format_bytes, parse_root_block_hardware};
use crate::verification::build_verification;

struct ShellCmd {
    cmd: &'static str,
    optional: bool,
}

const SHELL_COMMANDS: &[ShellCmd] = &[
    ShellCmd { cmd: "getprop", optional: false },
    ShellCmd { cmd: "dumpsys battery", optional: false },
    ShellCmd {
        cmd: "df -P -k /storage/emulated/0 /data 2>/dev/null; for d in /mnt/media_rw/*; do [ -d \"$d\" ] && df -P -k \"$d\"; done 2>/dev/null",
        optional: false,
    },
    ShellCmd {
        cmd: "dumpsys diskstats",
        optional: true,
    },
    ShellCmd {
        cmd: "ls /sys/block/ 2>/dev/null || true",
        optional: true,
    },
    ShellCmd {
        cmd: "cmd phone get-imei 0 2>/dev/null; echo ---IMEI2---; cmd phone get-imei 1 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "dumpsys telephony.registry 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "dumpsys iphonesubinfo 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "service call iphonesubinfo 1 i32 0 2>/dev/null; echo ---; service call iphonesubinfo 1 s16 com.android.shell 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "which su 2>/dev/null && echo found || echo notfound",
        optional: true,
    },
    ShellCmd {
        cmd: "pm list packages magisk 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "settings get global device_name",
        optional: true,
    },
    ShellCmd {
        cmd: "wm size 2>/dev/null; echo ---WM---; wm density 2>/dev/null; echo ---SETTINGS---; settings get secure bluetooth_address; settings get secure android_id; settings get system time_12_24; settings get secure wifi_mac_address; echo ---CPU---; nproc 2>/dev/null; cat /sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq 2>/dev/null; echo ---NET---; cat /sys/class/net/wlan0/address 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "dumpsys wifi 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "dumpsys fingerprint 2>/dev/null",
        optional: true,
    },
    ShellCmd {
        cmd: "service call iphonesubinfo 3 s16 com.android.shell 2>/dev/null",
        optional: true,
    },
];

fn run_shell_cmd(bridge: &AdbBridge, spec: &ShellCmd) -> Result<String, String> {
    match bridge.shell(spec.cmd) {
        Ok(out) => Ok(out),
        Err(_e) if spec.optional => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn load_device_summary(serial: &str) -> Result<DeviceSummary, String> {
    let bridge = AdbBridge::with_serial(serial).map_err(|e| e.to_string())?;

    let results: Vec<String> = SHELL_COMMANDS
        .iter()
        .map(|spec| run_shell_cmd(&bridge, spec))
        .collect::<Result<_, _>>()?;

    let props = parse_getprop(&results[0]);
    let mut dumpsys = parse_dumpsys_battery(&results[1]);

    let root_status = detect_root(&results[9], &results[10]);
    let rooted = is_rooted(&results[9], &results[10]);
    let sysfs_output = if rooted {
        run_shell_cmd(
            &bridge,
            &ShellCmd {
                cmd: ROOT_BATTERY_SHELL,
                optional: true,
            },
        )?
    } else {
        String::new()
    };
    let root_block_output = if rooted {
        run_shell_cmd(
            &bridge,
            &ShellCmd {
                cmd: ROOT_STORAGE_SHELL,
                optional: true,
            },
        )?
    } else {
        String::new()
    };

    let sysfs = parse_sysfs_battery(&sysfs_output);
    let block_hardware = parse_root_block_hardware(&root_block_output);
    merge_battery_sysfs(&mut dumpsys, &sysfs);
    let mut hardware = parse_hardware_shell(&results[12], &props);
    if hardware.wifi_mac.is_empty() {
        if let Some(mac) = parse_wifi_mac_from_dumpsys(&results[13]) {
            hardware.wifi_mac = mac;
        }
    }
    let fingerprint_status = parse_fingerprint_status(&results[14]).unwrap_or_default();
    let storage = build_storage_snapshot(
        &results[2],
        &results[3],
        &results[4],
        &props,
    );

    let imei_sources = [
        results[5].as_str(),
        results[6].as_str(),
        results[7].as_str(),
        results[8].as_str(),
    ];
    let (imei, imei2) = extract_imei(&imei_sources, &props);
    let imsi_sources = [
        results[6].as_str(),
        results[7].as_str(),
        results[15].as_str(),
    ];
    let imsi = extract_imsi(&imsi_sources, &props);

    let device_name = results[11].trim();
    let device_name = if device_name.is_empty() {
        get_prop(&props, "ro.product.model")
    } else {
        device_name.to_string()
    };

    let health = merged_health(&dumpsys, &sysfs);
    let is_charging = dumpsys.status == "Charging";
    let charge_cycles = if dumpsys.cycle_count > 0 {
        dumpsys.cycle_count
    } else {
        sysfs.cycle_count
    };

    let boot_state = get_prop(&props, "ro.boot.verifiedbootstate");
    let activation = if boot_state.to_lowercase() == "green" {
        "Activated"
    } else {
        "Active"
    };

    let region = resolve_sales_region(&props);
    let manufacturing_date = resolve_manufacturing_date(&props);
    let frp_status = detect_frp(&props);
    let bootloader_status = format_bootloader(&boot_state);
    let storage_total = format_bytes(storage.total);
    let storage_free = format_bytes(storage.free);

    let brand = get_prop(&props, "ro.product.brand");
    let model = get_prop(&props, "ro.product.model");
    let product = get_prop(&props, "ro.product.name");
    let android_version = get_prop(&props, "ro.build.version.release");
    let security_patch = get_prop(&props, "ro.build.version.security_patch");
    let serial = get_prop(&props, "ro.serialno");

    let details_input = DetailsInput {
        props: &props,
        battery: &dumpsys,
        hardware: &hardware,
        device_name: &device_name,
        brand: &brand,
        model: &model,
        product: &product,
        storage_total: &storage_total,
        region: &region,
        activation: &activation,
        manufacturing_date: &manufacturing_date,
        android_version: &android_version,
        security_patch: &security_patch,
        serial: &serial,
        imei: &imei,
        imei2: &imei2,
        imsi: &imsi,
        fingerprint_status: &fingerprint_status,
        rooted,
        root_status: &root_status,
        frp_status: &frp_status,
        bootloader_status: &bootloader_status,
        battery_health: health,
        charge_cycles,
    };

    let device_details = build_device_details(&details_input);
    let battery_details = build_battery_details(&dumpsys, health, charge_cycles, rooted);
    let storage_details = build_storage_details(
        &storage.breakdown,
        &storage_total,
        &storage_free,
        rooted,
        block_hardware.as_ref(),
    );
    let verification_checks = build_verification_checks(&details_input);
    let (verification_status, verification_score) = build_verification(&verification_checks);

    Ok(DeviceSummary {
        device_name,
        brand,
        model,
        product,
        region,
        activation_status: activation.to_string(),
        manufacturing_date,
        android_version,
        security_patch,
        serial,
        imei,
        imei2,
        root_status,
        frp_status,
        bootloader_status,
        storage_total,
        storage_free,
        storage_breakdown: storage.breakdown,
        battery_level: dumpsys.level,
        battery_health: health,
        battery_design_capacity_mah: if dumpsys.design_capacity > 0 {
            dumpsys.design_capacity
        } else {
            sysfs.design_capacity
        },
        battery_max_capacity_mah: if dumpsys.full_charge_capacity > 0 {
            dumpsys.full_charge_capacity
        } else {
            sysfs.full_charge_capacity
        },
        battery_temperature: format_temperature(dumpsys.temperature_tenths_c),
        battery_charging_power: dumpsys.charging_watts.clone(),
        battery_technology: if dumpsys.technology.is_empty() {
            "Unknown".to_string()
        } else {
            dumpsys.technology.clone()
        },
        charge_cycles,
        charging_status: dumpsys.status.clone(),
        is_charging,
        verification_status,
        verification_score,
        device_details,
        battery_details,
        storage_details,
        verification_checks,
    })
}
