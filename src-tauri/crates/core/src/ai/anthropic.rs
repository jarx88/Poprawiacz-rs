//! Anthropic Messages API, parity with `api_clients/anthropic_client.py`.
//! Requires explicit `max_tokens`. Non-streaming in MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::CorrectionRequest;

pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
pub const API_VERSION: &str = "2023-06-01";
pub const MAX_TOKENS: u32 = 2048;

pub fn build_body(req: &CorrectionRequest) -> Value {
    json!({
        "model": req.model,
        "max_tokens": MAX_TOKENS,
        "system": req.system_prompt(),
        "messages": [
            {"role": "user", "content": req.user_message()},
        ],
    })
}

/// Parse: first `text` block in `content[]`.
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    body.get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .or_else(|| blocks.first())
        })
        .and_then(|b| b.get("text"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| AiError::Response {
            provider: "anthropic".into(),
            status: None,
            message: "missing content[].text".into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::Provider;
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::Anthropic,
            model: "claude-3-7-sonnet-latest".into(),
            api_key: "sk-ant".into(),
            style: Style::Normal,
            text: "tekst".into(),
            stream: false,
        }
    }

    #[test]
    fn body_requires_max_tokens_and_system() {
        let b = build_body(&req());
        assert_eq!(b["max_tokens"], MAX_TOKENS);
        assert!(b["system"].as_str().unwrap().contains("virtual editor"));
        assert_eq!(b["messages"][0]["role"], "user");
    }

    #[test]
    fn parses_text_block() {
        let v = json!({"content":[{"type":"text","text":"wynik"}]});
        assert_eq!(parse_response(&v).unwrap(), "wynik");
    }

    #[test]
    fn skips_non_text_blocks() {
        let v = json!({"content":[{"type":"thinking","text":"hmm"},{"type":"text","text":"final"}]});
        assert_eq!(parse_response(&v).unwrap(), "final");
    }

    #[test]
    fn missing_content_errors() {
        assert!(parse_response(&json!({"content":[]})).is_err());
    }
}
