//! Google Gemini via REST `:generateContent`, behavioral parity with
//! `api_clients/gemini_client.py` (which used the Python SDK). The REST endpoint
//! and response schema are verified against the v1beta API. Non-streaming MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::{CorrectionRequest, ReasoningLevel};
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

/// Build the `thinkingConfig` for the model's generation. Gemini 2.5 uses an
/// integer `thinkingBudget` (0 = off; Pro has a 128 floor; -1 = dynamic), while
/// Gemini 3.x replaced it with an enum `thinkingLevel` and rejects the legacy
/// budget field. Off keeps quick edits fast (the whole point for this app).
fn thinking_config(level: ReasoningLevel, model: &str) -> Value {
    let m = model.to_ascii_lowercase();
    let is_pro = m.contains("pro");

    if m.contains("gemini-3") {
        let lvl = match level {
            // Gemini 3 Pro has no "minimal" tier; fall back to "low".
            ReasoningLevel::Off => {
                if is_pro {
                    "low"
                } else {
                    "minimal"
                }
            }
            ReasoningLevel::Low => "low",
            ReasoningLevel::Medium => "medium",
            ReasoningLevel::High | ReasoningLevel::Max => "high",
        };
        json!({ "thinkingLevel": lvl })
    } else {
        let budget: i32 = match level {
            ReasoningLevel::Off => {
                if is_pro {
                    128
                } else {
                    0
                }
            }
            ReasoningLevel::Low => 512,
            ReasoningLevel::Medium => 2048,
            ReasoningLevel::High => 8192,
            ReasoningLevel::Max => -1,
        };
        json!({ "thinkingBudget": budget })
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
            "thinkingConfig": thinking_config(req.reasoning_level, &req.model),
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
    use crate::ai::types::{Provider, ReasoningLevel};
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::Gemini,
            model: "gemini-2.5-flash".into(),
            api_key: "AIza".into(),
            style: Style::Normal,
            text: "tekst do poprawy".into(),
            stream: false,
            reasoning_level: ReasoningLevel::Off,
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
    fn v25_off_uses_budget_zero_flash_floor_pro() {
        let off = ReasoningLevel::Off;
        assert_eq!(thinking_config(off, "gemini-2.5-flash")["thinkingBudget"], 0);
        assert_eq!(thinking_config(off, "gemini-2.5-flash-lite")["thinkingBudget"], 0);
        // Pro cannot fully disable thinking -> 128 floor.
        assert_eq!(thinking_config(off, "gemini-2.5-pro")["thinkingBudget"], 128);
        let b = build_body(&req());
        assert_eq!(b["generationConfig"]["thinkingConfig"]["thinkingBudget"], 0);
    }

    #[test]
    fn v25_levels_map_to_budgets() {
        assert_eq!(thinking_config(ReasoningLevel::Low, "gemini-2.5-flash")["thinkingBudget"], 512);
        assert_eq!(thinking_config(ReasoningLevel::Medium, "gemini-2.5-flash")["thinkingBudget"], 2048);
        assert_eq!(thinking_config(ReasoningLevel::High, "gemini-2.5-flash")["thinkingBudget"], 8192);
        // Max -> dynamic.
        assert_eq!(thinking_config(ReasoningLevel::Max, "gemini-2.5-flash")["thinkingBudget"], -1);
    }

    #[test]
    fn gemini3_uses_thinking_level_not_budget() {
        let c = thinking_config(ReasoningLevel::Off, "gemini-3.5-flash");
        assert_eq!(c["thinkingLevel"], "minimal");
        assert!(c.get("thinkingBudget").is_none());
        // Gemini 3 Pro has no "minimal" -> falls back to "low".
        assert_eq!(thinking_config(ReasoningLevel::Off, "gemini-3-pro")["thinkingLevel"], "low");
        assert_eq!(thinking_config(ReasoningLevel::High, "gemini-3.5-flash")["thinkingLevel"], "high");
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
