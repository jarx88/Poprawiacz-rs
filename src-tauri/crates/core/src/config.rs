//! INI config parsing + migration, parity with `utils/config_manager.py`.
//!
//! The Python app stored everything (including plaintext API keys) in
//! `config.ini`. The new app keeps non-secret settings but migrates API keys
//! into the OS keychain (done in the app crate). This module only parses the
//! legacy file and resolves defaults; it never persists secrets.

use configparser::ini::Ini;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai::Provider;

/// Default model per provider (Python `DEFAULT_MODELS`).
pub fn default_model(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "gpt-5-mini",
        Provider::Anthropic => "claude-3-7-sonnet-latest",
        Provider::Gemini => "gemini-2.5-flash",
        Provider::DeepSeek => "deepseek-chat",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderModels {
    pub openai: Option<String>,
    pub anthropic: Option<String>,
    pub gemini: Option<String>,
    pub deepseek: Option<String>,
}

impl ProviderModels {
    /// Resolved model, falling back to the documented default.
    pub fn resolved(&self, provider: Provider) -> String {
        let chosen = match provider {
            Provider::OpenAI => &self.openai,
            Provider::Anthropic => &self.anthropic,
            Provider::Gemini => &self.gemini,
            Provider::DeepSeek => &self.deepseek,
        };
        chosen
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| default_model(provider).to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GeneralSettings {
    pub autostartup: bool,
    pub default_style: String,
    pub highlight_diffs: bool,
}

/// Result of parsing a legacy `config.ini`.
#[derive(Debug, Clone, Default)]
pub struct LegacyConfig {
    /// Plaintext API keys found in `[API_KEYS]` — to be migrated into the
    /// keychain and then dropped. Keyed by lowercase provider name.
    pub api_keys: HashMap<String, String>,
    pub models: ProviderModels,
    pub settings: GeneralSettings,
}

impl LegacyConfig {
    pub fn api_key(&self, provider: Provider) -> Option<&str> {
        self.api_keys.get(provider.key()).map(|s| s.as_str())
    }
}

fn truthy(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Parse a legacy INI string. `configparser` lowercases section/key names, which
/// gives us the case-insensitive lookup the Python `config_manager` emulated by
/// trying multiple case variants.
pub fn parse_ini(content: &str) -> Result<LegacyConfig, String> {
    let mut ini = Ini::new(); // case-insensitive (lowercases keys/sections)
    ini.read(content.to_string())?;

    let get = |section: &str, key: &str| -> Option<String> {
        ini.get(section, key).filter(|s| !s.trim().is_empty())
    };

    let mut api_keys = HashMap::new();
    for p in ["openai", "anthropic", "gemini", "deepseek"] {
        if let Some(k) = get("api_keys", p) {
            api_keys.insert(p.to_string(), k);
        }
    }

    let models = ProviderModels {
        openai: get("models", "openai"),
        anthropic: get("models", "anthropic"),
        gemini: get("models", "gemini"),
        deepseek: get("models", "deepseek"),
    };

    let settings = GeneralSettings {
        autostartup: get("settings", "autostartup").as_deref().map(truthy).unwrap_or(false),
        default_style: get("settings", "defaultstyle").unwrap_or_else(|| "normal".to_string()),
        highlight_diffs: get("settings", "highlightdiffs").as_deref().map(truthy).unwrap_or(false),
    };

    Ok(LegacyConfig { api_keys, models, settings })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[API_KEYS]\nopenai = sk-test-openai\nanthropic = sk-ant-test\ngemini = AIza-test\ndeepseek = sk-deep-test\n\n[MODELS]\nopenai = o4-mini\nanthropic = claude-sonnet-4-0\ngemini = gemini-2.5-flash\ndeepseek = deepseek-chat\n\n[SETTINGS]\nautostartup = 0\ndefaultstyle = normal\n";

    #[test]
    fn parses_keys_and_models() {
        let c = parse_ini(SAMPLE).unwrap();
        assert_eq!(c.api_key(Provider::OpenAI), Some("sk-test-openai"));
        assert_eq!(c.api_key(Provider::DeepSeek), Some("sk-deep-test"));
        assert_eq!(c.models.resolved(Provider::OpenAI), "o4-mini");
        assert_eq!(c.models.resolved(Provider::Anthropic), "claude-sonnet-4-0");
    }

    #[test]
    fn section_lookup_is_case_insensitive() {
        let upper = "[api_keys]\nOPENAI = sk-upper\n";
        let c = parse_ini(upper).unwrap();
        assert_eq!(c.api_key(Provider::OpenAI), Some("sk-upper"));
    }

    #[test]
    fn missing_model_falls_back_to_default() {
        let c = parse_ini("[API_KEYS]\nopenai = x\n").unwrap();
        assert_eq!(c.models.resolved(Provider::Gemini), "gemini-2.5-flash");
        assert_eq!(c.models.resolved(Provider::OpenAI), "gpt-5-mini");
    }

    #[test]
    fn empty_model_value_falls_back() {
        let c = parse_ini("[MODELS]\nopenai =   \n").unwrap();
        assert_eq!(c.models.resolved(Provider::OpenAI), "gpt-5-mini");
    }

    #[test]
    fn settings_truthiness() {
        let c = parse_ini("[SETTINGS]\nautostartup = 1\nhighlightdiffs = true\ndefaultstyle = professional\n").unwrap();
        assert!(c.settings.autostartup);
        assert!(c.settings.highlight_diffs);
        assert_eq!(c.settings.default_style, "professional");
    }

    #[test]
    fn defaults_when_section_absent() {
        let c = parse_ini("").unwrap();
        assert!(!c.settings.autostartup);
        assert_eq!(c.settings.default_style, "normal");
        assert!(c.api_keys.is_empty());
    }
}
