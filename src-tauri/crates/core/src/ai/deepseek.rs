//! DeepSeek (OpenAI-compatible Chat Completions), parity with
//! `api_clients/deepseek_client.py`. Uses the 35s timeout. Non-streaming MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::{CorrectionRequest, ReasoningLevel};

pub const ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
pub const MAX_TOKENS: u32 = 2000;

/// Only DeepSeek V4 models accept the `thinking` toggle. Older aliases
/// (`deepseek-chat`/`deepseek-reasoner`) have fixed behavior, so we omit the
/// field for them to avoid rejected requests.
fn supports_thinking_toggle(model: &str) -> bool {
    model.to_ascii_lowercase().contains("v4")
}

/// Map the unified level to DeepSeek V4: `None` => thinking disabled,
/// `Some(effort)` => thinking enabled with that `reasoning_effort`. DeepSeek
/// only exposes "high" and "max" (it folds low/medium into high), so Low–High
/// all map to "high".
fn thinking_effort(level: ReasoningLevel) -> Option<&'static str> {
    match level {
        ReasoningLevel::Off => None,
        ReasoningLevel::Max => Some("max"),
        _ => Some("high"),
    }
}

pub fn build_body(req: &CorrectionRequest) -> Value {
    let mut body = json!({
        "model": req.model,
        "messages": [
            {"role": "system", "content": req.system_prompt()},
            {"role": "user", "content": req.user_message()},
        ],
        "temperature": 0.7,
        "max_tokens": MAX_TOKENS,
        "stream": req.stream,
    });

    // Explicitly set thinking mode for V4 (the API recommends being explicit).
    // Disabling it is what keeps deepseek-v4-pro responsive for quick edits.
    if supports_thinking_toggle(&req.model) {
        match thinking_effort(req.reasoning_level) {
            None => body["thinking"] = json!({ "type": "disabled" }),
            Some(effort) => {
                body["thinking"] = json!({ "type": "enabled" });
                body["reasoning_effort"] = json!(effort);
            }
        }
    }

    body
}

/// Parse: `choices[0].message.content` (OpenAI-compatible).
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| AiError::Response {
            provider: "deepseek".into(),
            status: None,
            message: "missing choices[0].message.content".into(),
        })
}

/// Parse one SSE line (`data: {choices[0].delta.content}`), OpenAI-compatible.
pub fn parse_sse_line(line: &str) -> Result<Option<String>, AiError> {
    let line = line.trim();
    let Some(payload) = line.strip_prefix("data:") else {
        return Ok(None);
    };
    let payload = payload.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(payload).map_err(|e| AiError::Response {
        provider: "deepseek".into(),
        status: None,
        message: format!("bad SSE json: {e}"),
    })?;
    Ok(v
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::{Provider, ReasoningLevel};
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::DeepSeek,
            model: "deepseek-chat".into(),
            api_key: "sk-deep".into(),
            style: Style::Normal,
            text: "tekst".into(),
            stream: false,
            reasoning_level: ReasoningLevel::Off,
            verbosity: "medium".into(),
        }
    }

    #[test]
    fn no_thinking_field_for_legacy_models() {
        // deepseek-chat (non-v4 alias) must not carry the thinking toggle.
        let b = build_body(&req());
        assert!(b.get("thinking").is_none());
    }

    #[test]
    fn v4_off_disables_thinking() {
        let mut r = req();
        r.model = "deepseek-v4-pro".into();
        r.reasoning_level = ReasoningLevel::Off;
        let b = build_body(&r);
        assert_eq!(b["thinking"]["type"], "disabled");
        assert!(b.get("reasoning_effort").is_none());
    }

    #[test]
    fn v4_low_enables_thinking_high_effort() {
        let mut r = req();
        r.model = "deepseek-v4-flash".into();
        r.reasoning_level = ReasoningLevel::Low;
        let b = build_body(&r);
        assert_eq!(b["thinking"]["type"], "enabled");
        assert_eq!(b["reasoning_effort"], "high");
    }

    #[test]
    fn v4_max_enables_thinking_max_effort() {
        let mut r = req();
        r.model = "deepseek-v4-pro".into();
        r.reasoning_level = ReasoningLevel::Max;
        let b = build_body(&r);
        assert_eq!(b["thinking"]["type"], "enabled");
        assert_eq!(b["reasoning_effort"], "max");
    }

    #[test]
    fn sse_extracts_delta() {
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\"abc\"}}]}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("abc".to_string()));
    }

    #[test]
    fn body_has_temperature_and_max_tokens() {
        let b = build_body(&req());
        assert_eq!(b["temperature"], 0.7);
        assert_eq!(b["max_tokens"], MAX_TOKENS);
        assert_eq!(b["messages"][0]["role"], "system");
    }

    #[test]
    fn parses_content() {
        let v = json!({"choices":[{"message":{"content":"ok"}}]});
        assert_eq!(parse_response(&v).unwrap(), "ok");
    }
}
