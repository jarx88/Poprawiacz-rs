//! INI config parsing + migration, parity with `utils/config_manager.py`.
//!
//! The Python app stored everything (including plaintext API keys) in
//! `config.ini`. The new app keeps non-secret settings but migrates API keys
//! into the OS keychain (done in the app crate). This module only parses the
//! legacy file and resolves defaults; it never persists secrets.

use configparser::ini::Ini;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai::{Provider, ReasoningLevel};

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

/// Default clipboard processing delay (Python `ClipboardProcessingDelayMs`).
pub const DEFAULT_CLIPBOARD_DELAY_MS: u64 = 400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub autostartup: bool,
    pub default_style: String,
    pub highlight_diffs: bool,
    /// Total settle time (ms) for the copy-from-selection step.
    pub clipboard_delay_ms: u64,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            autostartup: false,
            default_style: String::new(),
            highlight_diffs: false,
            clipboard_delay_ms: DEFAULT_CLIPBOARD_DELAY_MS,
        }
    }
}

/// Unified per-provider reasoning strength. All default to `Off` — this is a
/// text-correction tool, and provider docs agree that this task class does not
/// need reasoning; the user raises a provider's level in Settings if they want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReasoningLevels {
    #[serde(default)]
    pub openai: ReasoningLevel,
    #[serde(default)]
    pub anthropic: ReasoningLevel,
    #[serde(default)]
    pub gemini: ReasoningLevel,
    #[serde(default)]
    pub deepseek: ReasoningLevel,
}

impl ReasoningLevels {
    pub fn for_provider(self, p: Provider) -> ReasoningLevel {
        match p {
            Provider::OpenAI => self.openai,
            Provider::Anthropic => self.anthropic,
            Provider::Gemini => self.gemini,
            Provider::DeepSeek => self.deepseek,
        }
    }
}

/// `[AI_SETTINGS]` — output verbosity (OpenAI Responses) plus the per-provider
/// reasoning strength.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    pub verbosity: String,
    #[serde(default)]
    pub reasoning_levels: ReasoningLevels,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            verbosity: "medium".to_string(),
            reasoning_levels: ReasoningLevels::default(),
        }
    }
}

/// Result of parsing a legacy `config.ini`.
#[derive(Debug, Clone, Default)]
pub struct LegacyConfig {
    /// Plaintext API keys found in `[API_KEYS]` — to be migrated into the
    /// keychain and then dropped. Keyed by lowercase provider name.
    pub api_keys: HashMap<String, String>,
    pub models: ProviderModels,
    pub settings: GeneralSettings,
    pub ai_settings: AiSettings,
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
        clipboard_delay_ms: get("settings", "clipboard_delay_ms")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_CLIPBOARD_DELAY_MS),
    };

    let ai_defaults = AiSettings::default();
    let level = |key: &str| get("reasoning_levels", key).map(|v| ReasoningLevel::parse(&v));
    // Carry a legacy [ai_settings] reasoningeffort over as the OpenAI level.
    let legacy_openai = get("ai_settings", "reasoningeffort").map(|v| ReasoningLevel::parse(&v));
    let reasoning_levels = ReasoningLevels {
        openai: level("openai").or(legacy_openai).unwrap_or_default(),
        anthropic: level("anthropic").unwrap_or_default(),
        gemini: level("gemini").unwrap_or_default(),
        deepseek: level("deepseek").unwrap_or_default(),
    };
    let ai_settings = AiSettings {
        verbosity: get("ai_settings", "verbosity").unwrap_or(ai_defaults.verbosity),
        reasoning_levels,
    };

    Ok(LegacyConfig { api_keys, models, settings, ai_settings })
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
        assert_eq!(c.settings.clipboard_delay_ms, 400);
        assert!(c.api_keys.is_empty());
        assert_eq!(c.ai_settings.reasoning_levels, ReasoningLevels::default());
        assert_eq!(c.ai_settings.reasoning_levels.openai, ReasoningLevel::Off);
        assert_eq!(c.ai_settings.verbosity, "medium");
    }

    #[test]
    fn parses_clipboard_delay() {
        let c = parse_ini("[SETTINGS]\nclipboard_delay_ms = 750\n").unwrap();
        assert_eq!(c.settings.clipboard_delay_ms, 750);
    }

    #[test]
    fn invalid_clipboard_delay_falls_back_to_default() {
        let c = parse_ini("[SETTINGS]\nclipboard_delay_ms = abc\n").unwrap();
        assert_eq!(c.settings.clipboard_delay_ms, 400);
    }

    #[test]
    fn parses_ai_settings() {
        let c = parse_ini("[AI_SETTINGS]\nreasoningeffort = low\nverbosity = high\n").unwrap();
        // Legacy reasoningeffort carries over as the OpenAI reasoning level.
        assert_eq!(c.ai_settings.reasoning_levels.openai, ReasoningLevel::Low);
        assert_eq!(c.ai_settings.verbosity, "high");
    }

    #[test]
    fn parses_per_provider_reasoning_levels() {
        let c = parse_ini("[REASONING_LEVELS]\nopenai = high\ndeepseek = max\ngemini = off\n").unwrap();
        assert_eq!(c.ai_settings.reasoning_levels.openai, ReasoningLevel::High);
        assert_eq!(c.ai_settings.reasoning_levels.deepseek, ReasoningLevel::Max);
        assert_eq!(c.ai_settings.reasoning_levels.gemini, ReasoningLevel::Off);
        // Unspecified provider defaults to Off.
        assert_eq!(c.ai_settings.reasoning_levels.anthropic, ReasoningLevel::Off);
    }
}
