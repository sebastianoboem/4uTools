use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub source: String,
    pub size: u64,
    pub category: String,
    pub rel_path: String,
    pub root_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriveInfo {
    pub device: String,
    pub mountpoint: String,
    pub fstype: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AndroidDevice {
    pub id: String,
    pub model: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: String,
    pub current_file: String,
    pub files_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyProgress {
    pub phase: String,
    pub current_file: String,
    pub copied_bytes: u64,
    pub total_bytes: u64,
    pub copied_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub copied_count: usize,
    pub destination: String,
    pub errors: Vec<String>,
}
