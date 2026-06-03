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

    /// All four providers stream (full parity with the Python app). Each
    /// provider has its own verified SSE parser in its module.
    pub fn supports_streaming(self) -> bool {
        true
    }

    /// OpenAI reasoning models (`gpt-5*`, `o1*`) use the Responses API instead
    /// of Chat Completions, with `reasoning.effort` / `text.verbosity`.
    pub fn uses_responses_api(model: &str) -> bool {
        let m = model.trim().to_ascii_lowercase();
        m.starts_with("gpt-5") || m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
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
    /// OpenAI Responses API reasoning effort: minimal|low|medium|high.
    pub reasoning_effort: String,
    /// OpenAI Responses API output verbosity: low|medium|high.
    pub verbosity: String,
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
    fn all_providers_stream() {
        for p in Provider::ALL {
            assert!(p.supports_streaming(), "{} should stream", p.display());
        }
    }

    #[test]
    fn responses_api_routing() {
        assert!(Provider::uses_responses_api("gpt-5-mini"));
        assert!(Provider::uses_responses_api("o1-preview"));
        assert!(Provider::uses_responses_api("o4-mini"));
        assert!(!Provider::uses_responses_api("gpt-4o-mini"));
        assert!(!Provider::uses_responses_api("claude-3-7-sonnet-latest"));
    }

    #[test]
    fn keys_are_lowercase_and_stable() {
        assert_eq!(Provider::OpenAI.key(), "openai");
        assert_eq!(Provider::DeepSeek.key(), "deepseek");
    }
}
