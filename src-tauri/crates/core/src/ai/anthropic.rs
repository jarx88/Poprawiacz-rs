//! Anthropic Messages API, parity with `api_clients/anthropic_client.py`.
//! Requires explicit `max_tokens`. Non-streaming in MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::{CorrectionRequest, ReasoningLevel};

pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
pub const API_VERSION: &str = "2023-06-01";
/// Token allowance for the actual answer (excludes the thinking budget).
pub const MAX_TOKENS: u32 = 2048;

/// Extended thinking exists on Claude 3.7 and all Claude 4.x. For other models
/// (e.g. claude-3-5-*) we omit the `thinking` block entirely.
fn supports_thinking(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    m.contains("claude-3-7")
        || m.contains("sonnet-4")
        || m.contains("opus-4")
        || m.contains("haiku-4")
}

/// Map the unified level to a `budget_tokens` value (`None` = no thinking).
/// Anthropic requires `budget_tokens >= 1024` and `< max_tokens`.
fn thinking_budget(level: ReasoningLevel) -> Option<u32> {
    match level {
        ReasoningLevel::Off => None,
        ReasoningLevel::Low => Some(1024),
        ReasoningLevel::Medium => Some(4096),
        ReasoningLevel::High => Some(10_000),
        ReasoningLevel::Max => Some(20_000),
    }
}

pub fn build_body(req: &CorrectionRequest) -> Value {
    let mut body = json!({
        "model": req.model,
        "max_tokens": MAX_TOKENS,
        "system": req.system_prompt(),
        "messages": [
            {"role": "user", "content": req.user_message()},
        ],
        "stream": req.stream,
    });

    if supports_thinking(&req.model) {
        if let Some(budget) = thinking_budget(req.reasoning_level) {
            // max_tokens must exceed budget_tokens, and the budget is consumed
            // on top of the answer — so raise the ceiling to fit both.
            body["max_tokens"] = json!(budget + MAX_TOKENS);
            body["thinking"] = json!({ "type": "enabled", "budget_tokens": budget });
        }
    }

    body
}

/// Parse one SSE line. Anthropic emits `event:`/`data:` pairs; we read the
/// `data:` JSON and extract `content_block_delta.delta.text`.
pub fn parse_sse_line(line: &str) -> Result<Option<String>, AiError> {
    let line = line.trim();
    let Some(payload) = line.strip_prefix("data:") else {
        return Ok(None);
    };
    let payload = payload.trim();
    if payload.is_empty() {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(payload).map_err(|e| AiError::Response {
        provider: "anthropic".into(),
        status: None,
        message: format!("bad SSE json: {e}"),
    })?;
    if v.get("type").and_then(Value::as_str) == Some("content_block_delta") {
        return Ok(v
            .pointer("/delta/text")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()));
    }
    Ok(None)
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
    use crate::ai::types::{Provider, ReasoningLevel};
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::Anthropic,
            model: "claude-3-7-sonnet-latest".into(),
            api_key: "sk-ant".into(),
            style: Style::Normal,
            text: "tekst".into(),
            stream: false,
            reasoning_level: ReasoningLevel::Off,
            verbosity: "medium".into(),
        }
    }

    #[test]
    fn off_has_no_thinking_block() {
        let b = build_body(&req());
        assert!(b.get("thinking").is_none());
        assert_eq!(b["max_tokens"], MAX_TOKENS);
    }

    #[test]
    fn thinking_enabled_raises_max_tokens() {
        let mut r = req();
        r.reasoning_level = ReasoningLevel::Low;
        let b = build_body(&r);
        assert_eq!(b["thinking"]["type"], "enabled");
        assert_eq!(b["thinking"]["budget_tokens"], 1024);
        // budget must stay below max_tokens.
        assert_eq!(b["max_tokens"], 1024 + MAX_TOKENS);
    }

    #[test]
    fn non_thinking_model_omits_block() {
        let mut r = req();
        r.model = "claude-3-5-haiku-latest".into();
        r.reasoning_level = ReasoningLevel::High;
        let b = build_body(&r);
        assert!(b.get("thinking").is_none());
    }

    #[test]
    fn sse_extracts_text_delta() {
        let line = "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"abc\"}}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("abc".to_string()));
    }

    #[test]
    fn sse_ignores_non_delta_events() {
        assert_eq!(parse_sse_line("data: {\"type\":\"message_start\"}").unwrap(), None);
        assert_eq!(parse_sse_line("event: ping").unwrap(), None);
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
