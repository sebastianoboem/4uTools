use std::collections::HashMap;

use crate::battery::{format_temperature, BatteryPartial};
use crate::getprop::get_prop;
use crate::hardware::{cpu_abi, soc_label, HardwarePartial};
use crate::models::{DetailRow, DetailSection, StorageBreakdown, VerificationCheck};
use crate::storage::{format_bytes, BlockHardwarePartial};

pub struct DetailsInput<'a> {
    pub props: &'a HashMap<String, String>,
    pub battery: &'a BatteryPartial,
    pub hardware: &'a HardwarePartial,
    pub device_name: &'a str,
    pub brand: &'a str,
    pub model: &'a str,
    pub product: &'a str,
    pub storage_total: &'a str,
    pub region: &'a str,
    pub activation: &'a str,
    pub manufacturing_date: &'a str,
    pub android_version: &'a str,
    pub security_patch: &'a str,
    pub serial: &'a str,
    pub imei: &'a str,
    pub imei2: &'a str,
    pub imsi: &'a str,
    pub fingerprint_status: &'a str,
    pub rooted: bool,
    pub root_status: &'a str,
    pub frp_status: &'a str,
    pub bootloader_status: &'a str,
    pub battery_health: u32,
    pub charge_cycles: u32,
}

fn row(label: &str, value: impl Into<String>) -> DetailRow {
    DetailRow {
        label: label.to_string(),
        value: value.into(),
        note: None,
    }
}

fn na(label: &str, reason: &str) -> DetailRow {
    DetailRow {
        label: label.to_string(),
        value: "N/A".to_string(),
        note: Some(reason.to_string()),
    }
}

pub fn build_device_details(input: &DetailsInput) -> Vec<DetailSection> {
    let p = input.props;
    let tz = get_prop(p, "persist.sys.timezone");
    let locale = get_prop(p, "persist.sys.locale");
    let baseband = get_prop(p, "gsm.version.baseband");
    let build_id = get_prop(p, "ro.build.display.id");
    let fingerprint = get_prop(p, "ro.build.fingerprint");
    let board = get_prop(p, "ro.product.board");
    let device_codename = get_prop(p, "ro.product.device");
    let sim_operator = get_prop(p, "gsm.sim.operator.alpha");
    let network_operator = get_prop(p, "gsm.operator.alpha");
    let color = get_prop(p, "ro.product.color");
    let capacity_label = if color == "N/A" {
        input.storage_total.to_string()
    } else {
        format!("{} · {}", input.storage_total, color)
    };

    let mut regional_rows = vec![
        row("Time Zone", tz),
        row("Region / Locale", locale),
        row("Remaining Battery", format!("{}%", input.battery.level)),
        row(
            "24-Hour Format",
            if input.hardware.time_format_24h.is_empty() {
                "Unknown".to_string()
            } else {
                input.hardware.time_format_24h.clone()
            },
        ),
        row("Battery Life (Health)", format!("{}%", input.battery_health)),
    ];
    if input.charge_cycles > 0 {
        regional_rows.insert(
            1,
            row("Charge Cycles", format!("{} Times", input.charge_cycles)),
        );
    }

    vec![
        DetailSection {
            title: "General".to_string(),
            rows: vec![
                row("Device Name", input.device_name),
                row("Model Name", input.model),
                row("Brand", input.brand),
                row("Product", input.product),
                row("Device Codename", device_codename),
                row("Serial Number", input.serial),
                row(
                    "Android ID",
                    if input.hardware.android_id.is_empty() {
                        "N/A".to_string()
                    } else {
                        input.hardware.android_id.clone()
                    },
                ),
                row("Capacity", capacity_label),
                row("Activation", input.activation),
                row("Root", input.root_status),
                row("Build ID", build_id),
                row("Security Patch", input.security_patch),
                row("Release / Mfg. Date", input.manufacturing_date),
                row("Sales Region", input.region),
                row("Model Number", input.model),
            ],
        },
        DetailSection {
            title: "Regional & Battery".to_string(),
            rows: regional_rows,
        },
        DetailSection {
            title: "Connectivity & SIM".to_string(),
            rows: vec![
                row("Baseband Version", baseband),
                row("IMEI 1", input.imei),
                row("IMEI 2", input.imei2),
                row(
                    "Wi-Fi Address",
                    if input.hardware.wifi_mac.is_empty() {
                        "N/A".to_string()
                    } else {
                        input.hardware.wifi_mac.clone()
                    },
                ),
                row(
                    "Bluetooth",
                    if input.hardware.bluetooth_mac.is_empty() {
                        "N/A".to_string()
                    } else {
                        input.hardware.bluetooth_mac.clone()
                    },
                ),
                if input.imsi == "N/A" {
                    na(
                        "IMSI",
                        "Richiede SIM attiva e permessi telephony (service call iphonesubinfo)",
                    )
                } else {
                    row("IMSI", input.imsi)
                },
                row("SIM Operator", sim_operator),
                row("Network Operator", network_operator),
                row("FRP / Google Lock", input.frp_status),
                row("Bootloader", input.bootloader_status),
            ],
        },
        DetailSection {
            title: "Hardware".to_string(),
            rows: vec![
                row("SoC / CPU", soc_label(p)),
                row(
                    "CPU Cores",
                    if input.hardware.cpu_cores > 0 {
                        input.hardware.cpu_cores.to_string()
                    } else {
                        "N/A".to_string()
                    },
                ),
                row(
                    "CPU Max Frequency",
                    if input.hardware.cpu_max_freq_mhz > 0 {
                        format!("{:.2} GHz", input.hardware.cpu_max_freq_mhz as f64 / 1000.0)
                    } else {
                        "N/A".to_string()
                    },
                ),
                row("CPU ABI", cpu_abi(p)),
                row("Board", board),
                row(
                    "Screen Resolution",
                    if input.hardware.screen_resolution.is_empty() {
                        "N/A".to_string()
                    } else {
                        input.hardware.screen_resolution.clone()
                    },
                ),
                row(
                    "Screen Density",
                    if input.hardware.screen_density.is_empty() {
                        "N/A".to_string()
                    } else {
                        format!("{} dpi", input.hardware.screen_density)
                    },
                ),
                row("OS Version", format!("Android {}", input.android_version)),
                row("Build Fingerprint", fingerprint),
            ],
        },
    ]
}

pub fn build_battery_details(
    battery: &BatteryPartial,
    health: u32,
    cycles: u32,
    rooted: bool,
) -> Vec<DetailSection> {
    let health_label = battery_health_label(battery);
    let voltage_v = battery.voltage_mv as f64 / 1000.0;
    let current_ma = battery.current_ua as f64 / 1000.0;

    let manufacturer = battery
        .manufacturer
        .as_deref()
        .filter(|s| !s.is_empty());
    let battery_sn = battery
        .serial_number
        .as_deref()
        .filter(|s| !s.is_empty());

    let mut spec_rows = vec![
        row(
            "Design Capacity",
            if battery.design_capacity > 0 {
                format!("{} mAh", battery.design_capacity)
            } else {
                "N/A".to_string()
            },
        ),
        row(
            "Current Capacity",
            if battery.charge_counter_mah > 0 {
                format!("{} mAh", battery.charge_counter_mah)
            } else {
                "N/A".to_string()
            },
        ),
        row(
            "Full Charge Capacity",
            if battery.full_charge_capacity > 0 {
                format!("{} mAh", battery.full_charge_capacity)
            } else {
                "N/A".to_string()
            },
        ),
        row(
            "Current Voltage",
            if battery.voltage_mv > 0 {
                format!("{voltage_v:.2} V")
            } else {
                "N/A".to_string()
            },
        ),
        row(
            "Battery Current",
            if battery.current_ua != 0 {
                format!("{current_ma:.0} mA")
            } else {
                "N/A".to_string()
            },
        ),
        row("Technology", &battery.technology),
        row("Capacity Level", battery.capacity_level_label.clone()),
        row(
            "At Warning Level",
            if battery.capacity_level >= 2 {
                "No"
            } else {
                "Yes"
            },
        ),
        row(
            "At Critical Level",
            if battery.capacity_level >= 1 {
                "No"
            } else {
                "Yes"
            },
        ),
    ];

    if rooted {
        if let Some(m) = manufacturer {
            spec_rows.insert(0, row("Battery Manufacturer", m));
        }
        if let Some(sn) = battery_sn {
            let pos = if manufacturer.is_some() { 1 } else { 0 };
            spec_rows.insert(pos, row("Battery SN", sn));
        }
    }

    let mut health_rows = vec![
        row("Battery Lifespan", format!("{health}%")),
        row("Battery Health", health_label),
    ];
    if cycles > 0 {
        health_rows.insert(1, row("Charge Cycles", cycles.to_string()));
    }

    vec![
        DetailSection {
            title: "Real-time Status".to_string(),
            rows: vec![
                row("Charging Status", &battery.charging_watts),
                row("Battery Level", format!("{}%", battery.level)),
                row("Temperature", format_temperature(battery.temperature_tenths_c)),
                row("Status", &battery.status),
            ],
        },
        DetailSection {
            title: "Health & Lifespan".to_string(),
            rows: health_rows,
        },
        DetailSection {
            title: "Specifications".to_string(),
            rows: spec_rows,
        },
    ]
}

fn battery_health_label(battery: &BatteryPartial) -> String {
    match battery.health_code {
        2 => "Good".to_string(),
        3 => "Overheat".to_string(),
        4 => "Dead".to_string(),
        5 => "Over voltage".to_string(),
        6 => "Unspecified failure".to_string(),
        7 => "Cold".to_string(),
        _ if battery.health > 0 => format!("{}% (estimated)", battery.health),
        _ => "Unknown".to_string(),
    }
}

pub fn build_storage_details(
    breakdown: &StorageBreakdown,
    storage_total: &str,
    storage_free: &str,
    rooted: bool,
    block_hardware: Option<&BlockHardwarePartial>,
) -> Vec<DetailSection> {
    let b = breakdown;
    let audio_video = b.audio + b.videos;
    let otg_row = if b.otg_total > 0 {
        row(
            "USB Drive (OTG)",
            format!(
                "{} used · {} free · {} total",
                format_bytes(b.otg_used),
                format_bytes(b.otg_free),
                format_bytes(b.otg_total),
            ),
        )
    } else {
        na(
            "USB Drive (OTG)",
            "Nessun volume OTG collegato (rilevato via df su /mnt/media_rw)",
        )
    };

    let mut sections = vec![DetailSection {
        title: "Usage Breakdown".to_string(),
        rows: vec![
            row("Free", storage_free),
            row("Total", storage_total),
            row("System", format_bytes(b.system)),
            row("Apps", format_bytes(b.apps)),
            row("Photos", format_bytes(b.photos)),
            row("Audio & Video", format_bytes(audio_video)),
            row("Downloads", format_bytes(b.downloads)),
            row("Other", format_bytes(b.other)),
            otg_row,
        ],
    }];

    if rooted {
        let hw_rows = if let Some(hw) = block_hardware {
            vec![
                row("Block Device", &hw.block),
                row(
                    "Vendor",
                    if hw.vendor.is_empty() {
                        "N/A".to_string()
                    } else {
                        hw.vendor.clone()
                    },
                ),
                row(
                    "Model",
                    if hw.model.is_empty() {
                        "N/A".to_string()
                    } else {
                        hw.model.clone()
                    },
                ),
                row(
                    "Serial Number",
                    if hw.serial.is_empty() {
                        "N/A".to_string()
                    } else {
                        hw.serial.clone()
                    },
                ),
            ]
        } else {
            vec![na(
                "Storage Hardware",
                "Sysfs block non leggibile anche con root su questo device",
            )]
        };
        sections.push(DetailSection {
            title: "Storage Hardware".to_string(),
            rows: hw_rows,
        });
    }

    sections
}

pub fn build_verification_checks(input: &DetailsInput) -> Vec<VerificationCheck> {
    let battery_sn = input
        .battery
        .serial_number
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("N/A");

    let mut checks = vec![
        VerificationCheck {
            item: "Model Name".to_string(),
            factory_value: None,
            read_value: input.model.to_string(),
            result: "Normal".to_string(),
            note: Some("Android non espone valore factory per confronto".to_string()),
        },
        VerificationCheck {
            item: "Storage Capacity".to_string(),
            factory_value: None,
            read_value: input.storage_total.to_string(),
            result: "Normal".to_string(),
            note: None,
        },
        VerificationCheck {
            item: "Sales Region".to_string(),
            factory_value: None,
            read_value: input.region.to_string(),
            result: if input.region == "N/A" {
                "Unknown".to_string()
            } else {
                "Normal".to_string()
            },
            note: None,
        },
        VerificationCheck {
            item: "Serial Number".to_string(),
            factory_value: None,
            read_value: input.serial.to_string(),
            result: "Normal".to_string(),
            note: None,
        },
        VerificationCheck {
            item: "IMEI".to_string(),
            factory_value: None,
            read_value: input.imei.to_string(),
            result: if input.imei.contains("restricted") {
                "Restricted".to_string()
            } else {
                "Normal".to_string()
            },
            note: None,
        },
        VerificationCheck {
            item: "Bootloader".to_string(),
            factory_value: Some("Locked".to_string()),
            read_value: input.bootloader_status.to_string(),
            result: "Normal".to_string(),
            note: None,
        },
        VerificationCheck {
            item: "Root Status".to_string(),
            factory_value: Some("No".to_string()),
            read_value: input.root_status.to_string(),
            result: if input.root_status.starts_with("Yes") {
                "Modified".to_string()
            } else {
                "Normal".to_string()
            },
            note: None,
        },
        VerificationCheck {
            item: "Battery Health".to_string(),
            factory_value: Some("100%".to_string()),
            read_value: format!("{}%", input.battery_health),
            result: if input.battery_health >= 80 {
                "Normal".to_string()
            } else if input.battery_health > 0 {
                "Degraded".to_string()
            } else {
                "Unknown".to_string()
            },
            note: None,
        },
    ];

    if !input.hardware.bluetooth_mac.is_empty() {
        checks.push(VerificationCheck {
            item: "Bluetooth".to_string(),
            factory_value: None,
            read_value: input.hardware.bluetooth_mac.clone(),
            result: "Normal".to_string(),
            note: None,
        });
    }
    if !input.hardware.wifi_mac.is_empty() {
        checks.push(VerificationCheck {
            item: "Wi-Fi Address".to_string(),
            factory_value: None,
            read_value: input.hardware.wifi_mac.clone(),
            result: "Normal".to_string(),
            note: None,
        });
    }

    checks.push(VerificationCheck {
        item: "Screen".to_string(),
        factory_value: None,
        read_value: input.hardware.screen_resolution.clone(),
        result: if input.hardware.screen_resolution.is_empty() {
            "Unknown".to_string()
        } else {
            "Normal".to_string()
        },
        note: None,
    });

    if !input.fingerprint_status.is_empty() {
        checks.push(VerificationCheck {
            item: "Fingerprint".to_string(),
            factory_value: None,
            read_value: input.fingerprint_status.to_string(),
            result: "Normal".to_string(),
            note: None,
        });
    }

    if input.rooted && battery_sn != "N/A" {
        checks.push(VerificationCheck {
            item: "Battery SN".to_string(),
            factory_value: None,
            read_value: battery_sn.to_string(),
            result: "Normal".to_string(),
            note: None,
        });
    }

    checks
}
