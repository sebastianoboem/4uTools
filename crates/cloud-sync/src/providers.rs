use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SYNC_SUBFOLDER: &str = "AndroidAdwareCleaner";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncProviderId {
    Local,
    Supabase,
    Custom,
}

impl SyncProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Supabase => "supabase",
            Self::Custom => "custom",
        }
    }

    pub fn from_str_id(s: &str) -> Self {
        match s {
            "supabase" => Self::Supabase,
            "custom" => Self::Custom,
            // google_drive, icloud, onedrive, dropbox: fornitori tolti.
            _ => Self::Local,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Local => "Solo locale",
            Self::Supabase => "Supabase",
            Self::Custom => "Cartella personalizzata",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProviderInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub detected_root: Option<String>,
    pub sync_folder: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncSettingsView {
    pub provider_id: String,
    pub sync_folder: Option<String>,
    pub subfolder: String,
    pub providers: Vec<SyncProviderInfo>,
    pub supabase_url: Option<String>,
    pub supabase_key: Option<String>,
}

pub fn list_providers() -> Vec<SyncProviderInfo> {
    let mut providers = vec![provider_supabase()];
    providers.extend([provider_local(), provider_custom()]);
    providers
}

fn provider_supabase() -> SyncProviderInfo {
    SyncProviderInfo {
        id: SyncProviderId::Supabase.as_str().into(),
        name: "Supabase".into(),
        description: "Il tuo progetto Supabase. Inserisci URL e chiave publishable. Ogni telefono ha un voto per app.".into(),
        detected_root: None,
        sync_folder: None,
        available: true,
    }
}

fn provider_local() -> SyncProviderInfo {
    SyncProviderInfo {
        id: SyncProviderId::Local.as_str().into(),
        name: "Solo locale".into(),
        description: "Nessuna sincronizzazione cloud — database solo su questo PC.".into(),
        detected_root: None,
        sync_folder: None,
        available: true,
    }
}

fn provider_custom() -> SyncProviderInfo {
    SyncProviderInfo {
        id: SyncProviderId::Custom.as_str().into(),
        name: "Cartella personalizzata".into(),
        description: "Scegli manualmente una cartella (anche su NAS o altro cloud).".into(),
        detected_root: None,
        sync_folder: None,
        available: true,
    }
}

pub fn resolve_sync_folder(provider_id: &str, custom_folder: Option<&str>) -> Option<PathBuf> {
    match SyncProviderId::from_str_id(provider_id) {
        SyncProviderId::Custom => custom_folder.map(PathBuf::from),
        SyncProviderId::Local | SyncProviderId::Supabase => None,
    }
}

pub fn ensure_sync_folder(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
