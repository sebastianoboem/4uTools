use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

static CATALOG: OnceLock<RwLock<BatteryCatalog>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryCatalogEntry {
    pub slug: String,
    pub marca: String,
    pub nome: String,
    pub batteria_mah: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatteryCatalogFile {
    pub version: u32,
    pub updated_at: String,
    pub source: String,
    pub entries: Vec<BatteryCatalogEntry>,
}

#[derive(Debug, Default)]
pub struct BatteryCatalog {
    entries: Vec<BatteryCatalogEntry>,
    /// normalized key → batteria_mah
    index: HashMap<String, u32>,
    updated_at: String,
}

impl BatteryCatalog {
    pub fn from_file(file: BatteryCatalogFile) -> Self {
        let mut index = HashMap::new();
        for e in &file.entries {
            if e.batteria_mah == 0 {
                continue;
            }
            for key in entry_keys(e) {
                index.entry(key).or_insert(e.batteria_mah);
            }
        }
        Self {
            entries: file.entries,
            index,
            updated_at: file.updated_at,
        }
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let file: BatteryCatalogFile =
            serde_json::from_slice(bytes).map_err(|e| format!("catalog JSON: {e}"))?;
        Ok(Self::from_file(file))
    }

    pub fn from_path(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read catalog: {e}"))?;
        Self::from_json(&bytes)
    }

    pub fn bundled() -> Self {
        const RAW: &str = include_str!("../../../resources/battery-catalog.json");
        Self::from_json(RAW.as_bytes()).unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn updated_at(&self) -> &str {
        &self.updated_at
    }

    /// Lookup design capacity (mAh) by device brand / model / product.
    pub fn lookup(&self, brand: &str, model: &str, product: &str) -> Option<u32> {
        let candidates = device_keys(brand, model, product);
        for key in &candidates {
            if let Some(&mah) = self.index.get(key) {
                return Some(mah);
            }
        }
        None
    }
}

fn catalog_lock() -> &'static RwLock<BatteryCatalog> {
    CATALOG.get_or_init(|| RwLock::new(BatteryCatalog::bundled()))
}

pub fn replace_battery_catalog(catalog: BatteryCatalog) {
    if let Ok(mut guard) = catalog_lock().write() {
        *guard = catalog;
    }
}

pub fn load_battery_catalog_path(path: &Path) -> Result<(), String> {
    let catalog = BatteryCatalog::from_path(path)?;
    replace_battery_catalog(catalog);
    Ok(())
}

pub fn lookup_design_capacity(brand: &str, model: &str, product: &str) -> Option<u32> {
    catalog_lock()
        .read()
        .ok()
        .and_then(|c| c.lookup(brand, model, product))
}

pub fn battery_catalog_stats() -> (usize, String) {
    catalog_lock()
        .read()
        .map(|c| (c.len(), c.updated_at().to_string()))
        .unwrap_or((0, String::new()))
}

pub fn normalize_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn entry_keys(e: &BatteryCatalogEntry) -> Vec<String> {
    let mut keys = Vec::new();
    let slug = normalize_key(&e.slug);
    let marca = normalize_key(&e.marca);
    let nome = normalize_key(&e.nome);
    let full = normalize_key(&format!("{} {}", e.marca, e.nome));

    if !slug.is_empty() {
        keys.push(slug.clone());
    }
    if !full.is_empty() {
        keys.push(full);
    }
    if !nome.is_empty() {
        keys.push(nome.clone());
        if !marca.is_empty() {
            keys.push(format!("{marca}{nome}"));
        }
    }
    // slug without leading brand token (poco-f4-gt → f4gt)
    if let Some(rest) = e.slug.split_once('-') {
        let rest_n = normalize_key(rest.1);
        if rest_n.len() >= 4 {
            keys.push(rest_n);
        }
    }
    keys
}

fn device_keys(brand: &str, model: &str, product: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let b = normalize_key(brand);
    let m = normalize_key(model);
    let p = normalize_key(product);

    if !m.is_empty() {
        keys.push(m.clone());
    }
    if !b.is_empty() && !m.is_empty() {
        keys.push(format!("{b}{m}"));
    }
    for variant in product_key_variants(&p) {
        keys.push(variant);
    }
    // Strip common brand prefixes duplicated in model ("POCO F4 GT" with brand Xiaomi/POCO)
    for prefix in [
        "poco", "redmi", "xiaomi", "samsung", "galaxy", "google", "pixel", "oneplus", "oppo",
        "realme", "vivo", "honor", "huawei", "motorola", "nokia", "asus", "nothing",
    ] {
        if let Some(rest) = m.strip_prefix(prefix) {
            if rest.len() >= 4 {
                keys.push(rest.to_string());
            }
        }
        if let Some(rest) = p.strip_prefix(prefix) {
            if rest.len() >= 4 {
                keys.push(rest.to_string());
                for variant in product_key_variants(rest) {
                    keys.push(variant);
                }
            }
        }
    }
    keys
}

/// Product names often include sales-region suffixes (`OnePlus8Pro_EEA` → `oneplus8proeea`).
fn product_key_variants(normalized_product: &str) -> Vec<String> {
    let mut out = Vec::new();
    if normalized_product.is_empty() {
        return out;
    }
    out.push(normalized_product.to_string());
    const REGION_SUFFIXES: &[&str] = &[
        "eea", "eur", "eu", "global", "glo", "row", "china", "cn", "in", "india", "na", "usa",
        "us", "jp", "kr", "tw", "hk", "ru", "tr", "latam", "mea",
    ];
    for suf in REGION_SUFFIXES {
        if let Some(rest) = normalized_product.strip_suffix(suf) {
            if rest.len() >= 4 {
                out.push(rest.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> BatteryCatalog {
        BatteryCatalog::from_file(BatteryCatalogFile {
            version: 1,
            updated_at: "test".into(),
            source: "test".into(),
            entries: vec![
                BatteryCatalogEntry {
                    slug: "poco-f4-gt".into(),
                    marca: "POCO".into(),
                    nome: "F4 GT".into(),
                    batteria_mah: 4700,
                },
                BatteryCatalogEntry {
                    slug: "samsung-galaxy-s24".into(),
                    marca: "Samsung".into(),
                    nome: "Galaxy S24".into(),
                    batteria_mah: 4000,
                },
                BatteryCatalogEntry {
                    slug: "xiaomi-12x".into(),
                    marca: "Xiaomi".into(),
                    nome: "12X".into(),
                    batteria_mah: 4500,
                },
            ],
        })
    }

    #[test]
    fn finds_poco_by_marketing_name() {
        let c = sample_catalog();
        assert_eq!(c.lookup("Xiaomi", "POCO F4 GT", "ingres"), Some(4700));
        assert_eq!(c.lookup("POCO", "F4 GT", "ingres"), Some(4700));
    }

    #[test]
    fn finds_samsung_s24_not_ultra() {
        let c = sample_catalog();
        assert_eq!(c.lookup("samsung", "SM-S921B", "e1s"), None);
        assert_eq!(c.lookup("samsung", "Galaxy S24", "e1s"), Some(4000));
    }

    #[test]
    fn finds_xiaomi_12x() {
        let c = sample_catalog();
        assert_eq!(c.lookup("Xiaomi", "2206123SC", "psyche"), None);
        assert_eq!(c.lookup("Xiaomi", "12X", "psyche"), Some(4500));
    }

    #[test]
    fn finds_oneplus_8_pro_from_product_codename() {
        let c = BatteryCatalog::from_file(BatteryCatalogFile {
            version: 1,
            updated_at: "test".into(),
            source: "test".into(),
            entries: vec![BatteryCatalogEntry {
                slug: "oneplus-8-pro".into(),
                marca: "OnePlus".into(),
                nome: "8 Pro".into(),
                batteria_mah: 4510,
            }],
        });
        // ADB model is IN2023; marketing name is only in product.
        assert_eq!(
            c.lookup("OnePlus", "IN2023", "OnePlus8Pro_EEA"),
            Some(4510)
        );
        assert_eq!(c.lookup("OnePlus", "IN2023", "OnePlus8Pro"), Some(4510));
        assert_eq!(c.lookup("OnePlus", "IN2023", "lemonade"), None);
    }
}
