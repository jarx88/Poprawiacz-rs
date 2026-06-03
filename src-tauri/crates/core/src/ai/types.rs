//! Shared AI types & constants, parity with `api_clients/base_client.py`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::prompts::Style;

/// Connection timeout for every provider (`connect=8s` in Python).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// Standard read timeout (`DEFAULT_TIMEOUT = 25`).
pub const STANDARD_TIMEOUT: Duration = Duration::from_secs(25);
/// DeepSeek read timeout (`DEEPSEEK_TIMEOUT = 35`). MUST stay >= 30s.
pub const DEEPSEEK_TIMEOUT: Duration = Duration::from_secs(35);
/// Retry attempts after the first try (`DEFAULT_RETRIES = 2`).
pub const MAX_RETRIES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::OpenAI,
        Provider::Anthropic,
        Provider::Gemini,
        Provider::DeepSeek,
    ];

    /// Lowercase config/keychain key.
    pub fn key(self) -> &'static str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Gemini => "gemini",
            Provider::DeepSeek => "deepseek",
        }
    }

    /// Human-readable name shown in the UI / panel header.
    pub fn display(self) -> &'static str {
        match self {
            Provider::OpenAI => "OpenAI",
            Provider::Anthropic => "Anthropic",
            Provider::Gemini => "Gemini",
            Provider::DeepSeek => "DeepSeek",
        }
    }

    /// Read timeout. DeepSeek is empirically slower and gets 35s.
    pub fn timeout(self) -> Duration {
        match self {
            Provider::DeepSeek => DEEPSEEK_TIMEOUT,
            _ => STANDARD_TIMEOUT,
        }
    }

    /// Only OpenAI streams in the MVP (Python streamed more, but we ship
    /// streaming only where it's verified — see plan "DO NOT" rules).
    pub fn supports_streaming(self) -> bool {
        matches!(self, Provider::OpenAI)
    }
}

/// Everything needed to issue one correction call to one provider.
#[derive(Debug, Clone)]
pub struct CorrectionRequest {
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
    pub style: Style,
    pub text: String,
    pub stream: bool,
}

impl CorrectionRequest {
    pub fn system_prompt(&self) -> &'static str {
        crate::prompts::system_prompt(self.style)
    }
    pub fn user_message(&self) -> String {
        crate::prompts::user_message(self.style, &self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_timeout_never_below_30s() {
        assert!(Provider::DeepSeek.timeout() >= Duration::from_secs(30));
        assert_eq!(Provider::DeepSeek.timeout(), Duration::from_secs(35));
    }

    #[test]
    fn standard_providers_use_25s() {
        for p in [Provider::OpenAI, Provider::Anthropic, Provider::Gemini] {
            assert_eq!(p.timeout(), Duration::from_secs(25));
        }
    }

    #[test]
    fn only_openai_streams_in_mvp() {
        assert!(Provider::OpenAI.supports_streaming());
        assert!(!Provider::Anthropic.supports_streaming());
        assert!(!Provider::Gemini.supports_streaming());
        assert!(!Provider::DeepSeek.supports_streaming());
    }

    #[test]
    fn keys_are_lowercase_and_stable() {
        assert_eq!(Provider::OpenAI.key(), "openai");
        assert_eq!(Provider::DeepSeek.key(), "deepseek");
    }
}
