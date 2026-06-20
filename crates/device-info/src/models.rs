use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBreakdown {
    pub system: u64,
    pub apps: u64,
    pub photos: u64,
    pub audio: u64,
    pub videos: u64,
    pub downloads: u64,
    pub other: u64,
    pub free: u64,
    pub total: u64,
    #[serde(default)]
    pub otg_total: u64,
    #[serde(default)]
    pub otg_used: u64,
    #[serde(default)]
    pub otg_free: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailSection {
    pub title: String,
    pub rows: Vec<DetailRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCheck {
    pub item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factory_value: Option<String>,
    pub read_value: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub device_name: String,
    pub brand: String,
    pub model: String,
    pub product: String,
    pub region: String,
    pub activation_status: String,
    pub manufacturing_date: String,
    pub android_version: String,
    pub security_patch: String,
    pub serial: String,
    pub imei: String,
    pub imei2: String,
    pub root_status: String,
    pub frp_status: String,
    pub bootloader_status: String,
    pub storage_total: String,
    pub storage_free: String,
    pub storage_breakdown: StorageBreakdown,
    pub battery_level: u32,
    pub battery_health: u32,
    pub battery_design_capacity_mah: u32,
    pub battery_max_capacity_mah: u32,
    pub battery_temperature: String,
    pub battery_charging_power: String,
    pub battery_technology: String,
    pub charge_cycles: u32,
    pub charging_status: String,
    pub is_charging: bool,
    pub verification_status: String,
    pub verification_score: u32,
    pub device_details: Vec<DetailSection>,
    pub battery_details: Vec<DetailSection>,
    pub storage_details: Vec<DetailSection>,
    pub verification_checks: Vec<VerificationCheck>,
}
