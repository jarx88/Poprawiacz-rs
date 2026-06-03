//! Google Gemini via REST `:generateContent`, behavioral parity with
//! `api_clients/gemini_client.py` (which used the Python SDK). The REST endpoint
//! and response schema are verified against the v1beta API. Non-streaming MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::CorrectionRequest;
use crate::prompts::instruction_prompt;

pub const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
pub const MAX_OUTPUT_TOKENS: u32 = 3072;

/// Full endpoint for a model, e.g. `.../models/gemini-2.5-flash:generateContent`.
pub fn endpoint(model: &str) -> String {
    format!("{BASE}/{model}:generateContent")
}

/// Streaming endpoint (Server-Sent Events).
pub fn stream_endpoint(model: &str) -> String {
    format!("{BASE}/{model}:streamGenerateContent?alt=sse")
}

/// Parse one SSE `data:` line of a streaming response into the incremental
/// text of the first candidate.
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
        provider: "gemini".into(),
        status: None,
        message: format!("bad SSE json: {e}"),
    })?;
    let text: String = v
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default();
    Ok(if text.is_empty() { None } else { Some(text) })
}

fn safety_settings() -> Value {
    json!([
        {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
        {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
        {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
        {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"},
    ])
}

/// "Thinking" budget (parity with Python): disabled (0) for Flash/Lite, 128 for
/// Pro (which cannot fully disable it). Disabling thinking is what makes Gemini
/// 2.5 respond quickly instead of pausing to "think".
fn thinking_budget(model: &str) -> i32 {
    if model.to_ascii_lowercase().contains("pro") {
        128
    } else {
        0
    }
}

pub fn build_body(req: &CorrectionRequest) -> Value {
    json!({
        "system_instruction": {"parts": [{"text": req.system_prompt()}]},
        "contents": [{
            "role": "user",
            "parts": [
                {"text": instruction_prompt(req.style)},
                {"text": req.text},
            ],
        }],
        "generationConfig": {
            "maxOutputTokens": MAX_OUTPUT_TOKENS,
            "temperature": 0.7,
            "topP": 0.9,
            "topK": 32,
            "thinkingConfig": { "thinkingBudget": thinking_budget(&req.model) },
        },
        "safetySettings": safety_settings(),
    })
}

/// Concatenate all `text` parts of the first candidate.
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    let parts = body
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .ok_or_else(|| AiError::Response {
            provider: "gemini".into(),
            status: None,
            message: "missing candidates[0].content.parts".into(),
        })?;
    let text: String = parts
        .iter()
        .filter_map(|p| p.get("text").and_then(Value::as_str))
        .collect();
    if text.is_empty() {
        return Err(AiError::Response {
            provider: "gemini".into(),
            status: None,
            message: "empty candidate text".into(),
        });
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::Provider;
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::Gemini,
            model: "gemini-2.5-flash".into(),
            api_key: "AIza".into(),
            style: Style::Normal,
            text: "tekst do poprawy".into(),
            stream: false,
            reasoning_effort: "high".into(),
            verbosity: "medium".into(),
        }
    }

    #[test]
    fn stream_endpoint_uses_sse() {
        assert!(stream_endpoint("gemini-2.5-flash")
            .ends_with("models/gemini-2.5-flash:streamGenerateContent?alt=sse"));
    }

    #[test]
    fn sse_extracts_parts_text() {
        let line = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"abc\"}]}}]}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("abc".to_string()));
    }

    #[test]
    fn endpoint_includes_model_and_method() {
        assert_eq!(
            endpoint("gemini-2.5-flash"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn body_has_system_instruction_and_safety() {
        let b = build_body(&req());
        assert!(b["system_instruction"]["parts"][0]["text"].as_str().unwrap().contains("virtual editor"));
        assert_eq!(b["contents"][0]["parts"][1]["text"], "tekst do poprawy");
        assert_eq!(b["safetySettings"][0]["threshold"], "BLOCK_NONE");
        assert_eq!(b["generationConfig"]["maxOutputTokens"], MAX_OUTPUT_TOKENS);
    }

    #[test]
    fn thinking_disabled_for_flash_enabled_for_pro() {
        assert_eq!(thinking_budget("gemini-2.5-flash"), 0);
        assert_eq!(thinking_budget("gemini-2.5-flash-lite"), 0);
        assert_eq!(thinking_budget("gemini-2.5-pro"), 128);
        let b = build_body(&req());
        assert_eq!(b["generationConfig"]["thinkingConfig"]["thinkingBudget"], 0);
    }

    #[test]
    fn parses_concatenated_parts() {
        let v = json!({"candidates":[{"content":{"parts":[{"text":"abc"},{"text":"def"}]}}]});
        assert_eq!(parse_response(&v).unwrap(), "abcdef");
    }

    #[test]
    fn missing_candidates_errors() {
        assert!(parse_response(&json!({"candidates":[]})).is_err());
    }
}
