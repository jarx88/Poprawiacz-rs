//! Settings persistence (non-secret JSON) + API keys in the OS keychain.

use std::fs;
use std::path::PathBuf;

use poprawiacz_core::ai::Provider;
use poprawiacz_core::config::{parse_ini, ProviderModels};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYCHAIN_SERVICE: &str = "PoprawiaczTekstu";

/// Non-secret settings persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub models: ProviderModels,
    pub default_style: String,
}

impl AppSettings {
    fn ensure_style(mut self) -> Self {
        if self.default_style.trim().is_empty() {
            self.default_style = "normal".to_string();
        }
        self
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no app config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let Ok(path) = settings_path(app) else {
        return AppSettings::default().ensure_style();
    };
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<AppSettings>(&s)
            .unwrap_or_default()
            .ensure_style(),
        Err(_) => AppSettings::default().ensure_style(),
    }
}

pub fn persist_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

// --- Keychain (secrets) -----------------------------------------------------

fn entry(provider: Provider) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, provider.key()).map_err(|e| e.to_string())
}

pub fn get_api_key(provider: Provider) -> Option<String> {
    entry(provider)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.trim().is_empty())
}

pub fn set_api_key(provider: Provider, key: &str) -> Result<(), String> {
    let e = entry(provider)?;
    if key.trim().is_empty() {
        // empty => clear
        let _ = e.delete_credential();
        Ok(())
    } else {
        e.set_password(key).map_err(|err| err.to_string())
    }
}

fn has_api_key(provider: Provider) -> bool {
    get_api_key(provider).is_some()
}

// --- Tauri commands ---------------------------------------------------------

/// Snapshot returned to the frontend. Never includes raw key material — only
/// whether a key is present.
#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub models: ProviderModels,
    pub default_style: String,
    /// provider key -> whether a key is stored
    pub keys_present: std::collections::HashMap<String, bool>,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    let s = load_settings(&app);
    let mut keys_present = std::collections::HashMap::new();
    for p in Provider::ALL {
        keys_present.insert(p.key().to_string(), has_api_key(p));
    }
    SettingsView {
        models: s.models,
        default_style: s.default_style,
        keys_present,
    }
}

/// Payload for saving settings. Keys are optional; a present non-empty value is
/// written to the keychain, an empty string clears it, `None` leaves it intact.
#[derive(Debug, Deserialize)]
pub struct SaveSettingsPayload {
    pub models: ProviderModels,
    pub default_style: String,
    #[serde(default)]
    pub api_keys: std::collections::HashMap<String, String>,
}

#[tauri::command]
pub fn save_settings(app: AppHandle, payload: SaveSettingsPayload) -> Result<(), String> {
    save_settings_inner(&app, &payload)
}

fn save_settings_inner(app: &AppHandle, payload: &SaveSettingsPayload) -> Result<(), String> {
    let settings = AppSettings {
        models: payload.models.clone(),
        default_style: payload.default_style.clone(),
    }
    .ensure_style();
    persist_settings(app, &settings)?;
    for p in Provider::ALL {
        if let Some(k) = payload.api_keys.get(p.key()) {
            set_api_key(p, k)?;
        }
    }
    Ok(())
}

/// Migrate a legacy `config.ini`: persist models/style and move API keys into
/// the keychain. The plaintext keys are NOT written back to disk.
#[tauri::command]
pub fn migrate_config_ini(app: AppHandle, path: String) -> Result<u32, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let legacy = parse_ini(&content)?;

    let settings = AppSettings {
        models: legacy.models.clone(),
        default_style: legacy.settings.default_style.clone(),
    }
    .ensure_style();
    persist_settings(&app, &settings)?;

    let mut migrated = 0u32;
    for p in Provider::ALL {
        if let Some(k) = legacy.api_key(p) {
            set_api_key(p, k)?;
            migrated += 1;
        }
    }
    Ok(migrated)
}
