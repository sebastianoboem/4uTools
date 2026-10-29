use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::state::GphotosState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionParams {
    pub click_delay: f64,
    pub scroll_fraction: f64,
    pub settle_delay: f64,
    pub passes_per_step: f64,
    pub stall_passes: f64,
    pub skip_label_prefix: String,
    pub max_selections: f64,
    pub download_after_selection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub dev_mode: bool,
    pub tutorial_completed: bool,
}

impl Default for SelectionParams {
    fn default() -> Self {
        Self {
            click_delay: 80.0,
            scroll_fraction: 70.0,
            settle_delay: 1200.0,
            passes_per_step: 2.0,
            stall_passes: 6.0,
            skip_label_prefix: "Salva tutte le foto".into(),
            max_selections: 0.0,
            download_after_selection: false,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dev_mode: false,
            tutorial_completed: false,
        }
    }
}

pub fn clamp_params(mut params: SelectionParams) -> SelectionParams {
    params.click_delay = params.click_delay.clamp(0.0, 500.0);
    params.scroll_fraction = if params.scroll_fraction > 80.0 {
        70.0
    } else {
        params.scroll_fraction.clamp(20.0, 80.0)
    };
    params.settle_delay = params.settle_delay.clamp(200.0, 5000.0);
    params.passes_per_step = params.passes_per_step.clamp(1.0, 3.0);
    params.stall_passes = params.stall_passes.clamp(3.0, 15.0);
    params.max_selections = params.max_selections.clamp(0.0, 1_000_000.0);
    if params.skip_label_prefix.trim().is_empty() {
        params.skip_label_prefix = SelectionParams::default().skip_label_prefix;
    } else {
        params.skip_label_prefix = params.skip_label_prefix.trim().to_string();
    }
    params
}

fn read_json(path: &PathBuf) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

pub fn load_settings(state: &GphotosState) -> SelectionParams {
    let path = state.root.join("settings.json");
    let Some(raw) = read_json(&path) else {
        return SelectionParams::default();
    };
    let mut params = SelectionParams::default();
    if let Ok(parsed) = serde_json::from_value::<SelectionParams>(raw) {
        params = parsed;
    }
    clamp_params(params)
}

pub fn save_settings(state: &GphotosState, params: SelectionParams) -> Result<SelectionParams, String> {
    let params = clamp_params(params);
    write_json(&state.root.join("settings.json"), &params)?;
    Ok(params)
}

pub fn load_app_settings(state: &GphotosState) -> AppSettings {
    let path = state.root.join("app-settings.json");
    read_json(&path)
        .and_then(|raw| serde_json::from_value(raw).ok())
        .unwrap_or_default()
}

pub fn save_app_settings(state: &GphotosState, settings: &AppSettings) -> Result<(), String> {
    write_json(&state.root.join("app-settings.json"), settings)
}
