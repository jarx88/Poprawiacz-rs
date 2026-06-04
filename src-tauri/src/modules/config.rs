//! Settings persistence (non-secret JSON) + API keys in the OS keychain.

use std::fs;
use std::path::PathBuf;

use poprawiacz_core::ai::Provider;
use poprawiacz_core::config::{parse_ini, AiSettings, ProviderModels};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYCHAIN_SERVICE: &str = "PoprawiaczTekstu";

/// Default clipboard processing delay (parity with Python `clipboard_delay_ms`).
const DEFAULT_CLIPBOARD_DELAY_MS: u64 = 400;

fn default_clipboard_delay_ms() -> u64 {
    DEFAULT_CLIPBOARD_DELAY_MS
}

/// Non-secret settings persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub models: ProviderModels,
    pub default_style: String,
    #[serde(default)]
    pub highlight_diffs: bool,
    #[serde(default)]
    pub autostartup: bool,
    #[serde(default)]
    pub ai_settings: AiSettings,
    #[serde(default = "default_clipboard_delay_ms")]
    pub clipboard_delay_ms: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            models: ProviderModels::default(),
            default_style: String::new(),
            highlight_diffs: false,
            autostartup: false,
            ai_settings: AiSettings::default(),
            clipboard_delay_ms: DEFAULT_CLIPBOARD_DELAY_MS,
        }
    }
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

fn entry(provider: Provider) -> Result<keyring_core::Entry, String> {
    keyring_core::Entry::new(KEYCHAIN_SERVICE, provider.key()).map_err(|e| e.to_string())
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
    pub highlight_diffs: bool,
    pub autostartup: bool,
    pub ai_settings: AiSettings,
    pub clipboard_delay_ms: u64,
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
        highlight_diffs: s.highlight_diffs,
        autostartup: s.autostartup || super::autostart::is_enabled(),
        ai_settings: s.ai_settings,
        clipboard_delay_ms: s.clipboard_delay_ms,
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
    pub highlight_diffs: bool,
    #[serde(default)]
    pub autostartup: bool,
    #[serde(default)]
    pub ai_settings: AiSettings,
    #[serde(default = "default_clipboard_delay_ms")]
    pub clipboard_delay_ms: u64,
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
        highlight_diffs: payload.highlight_diffs,
        autostartup: payload.autostartup,
        ai_settings: payload.ai_settings.clone(),
        clipboard_delay_ms: payload.clipboard_delay_ms,
    }
    .ensure_style();
    persist_settings(app, &settings)?;
    for p in Provider::ALL {
        if let Some(k) = payload.api_keys.get(p.key()) {
            set_api_key(p, k)?;
        }
    }
    // Reflect autostart preference into the OS (Windows registry).
    if let Err(e) = super::autostart::set_enabled(payload.autostartup) {
        tracing::warn!("autostart update failed: {e}");
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
        highlight_diffs: legacy.settings.highlight_diffs,
        autostartup: legacy.settings.autostartup,
        ai_settings: legacy.ai_settings.clone(),
        clipboard_delay_ms: legacy.settings.clipboard_delay_ms,
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
