//! OpenAI Chat Completions, parity with `api_clients/openai_client.py`.
//! The only provider with streaming in the MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::CorrectionRequest;

pub const ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// Build the JSON request body.
pub fn build_body(req: &CorrectionRequest) -> Value {
    json!({
        "model": req.model,
        "messages": [
            {"role": "system", "content": req.system_prompt()},
            {"role": "user", "content": req.user_message()},
        ],
        "stream": req.stream,
    })
}

/// Parse a non-streaming response: `choices[0].message.content`.
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| AiError::Response {
            provider: "openai".into(),
            status: None,
            message: "missing choices[0].message.content".into(),
        })
}

/// Parse one SSE line into a content delta. Returns:
/// - `Ok(Some(text))` for a content chunk,
/// - `Ok(None)` for keep-alives / non-content events / `[DONE]`,
/// - `Err` only if a data line is present but malformed.
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
        provider: "openai".into(),
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
    use crate::prompts::Style;

    fn req(stream: bool) -> CorrectionRequest {
        CorrectionRequest {
            provider: super::super::types::Provider::OpenAI,
            model: "gpt-5-mini".into(),
            api_key: "sk-x".into(),
            style: Style::Normal,
            text: "helo".into(),
            stream,
        }
    }

    #[test]
    fn body_has_system_and_user_messages() {
        let b = build_body(&req(true));
        assert_eq!(b["model"], "gpt-5-mini");
        assert_eq!(b["stream"], true);
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][1]["role"], "user");
        assert!(b["messages"][1]["content"].as_str().unwrap().contains("helo"));
    }

    #[test]
    fn parses_non_stream_content() {
        let v = json!({"choices":[{"message":{"content":"poprawiony"}}]});
        assert_eq!(parse_response(&v).unwrap(), "poprawiony");
    }

    #[test]
    fn missing_content_is_response_error() {
        let v = json!({"choices":[]});
        assert!(parse_response(&v).is_err());
    }

    #[test]
    fn sse_extracts_delta() {
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\"abc\"}}]}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("abc".to_string()));
    }

    #[test]
    fn sse_done_and_empty_are_none() {
        assert_eq!(parse_sse_line("data: [DONE]").unwrap(), None);
        assert_eq!(parse_sse_line("").unwrap(), None);
        assert_eq!(parse_sse_line(": keep-alive").unwrap(), None);
        assert_eq!(parse_sse_line("data: {\"choices\":[{\"delta\":{}}]}").unwrap(), None);
    }

    #[test]
    fn sse_bad_json_errors() {
        assert!(parse_sse_line("data: {not json").is_err());
    }
}
