//! OpenAI: Chat Completions for standard models and the Responses API for
//! reasoning models (`gpt-5*`, `o1*`, `o3*`, `o4*`). Parity with
//! `api_clients/openai_client.py`. Streaming supported on both paths.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::{CorrectionRequest, Provider, ReasoningLevel};

pub const CHAT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
pub const RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
pub const MAX_OUTPUT_TOKENS: u32 = 2000;

/// Whether this request goes to the Responses API.
pub fn is_responses(req: &CorrectionRequest) -> bool {
    Provider::uses_responses_api(&req.model)
}

/// Endpoint for a model.
pub fn endpoint(model: &str) -> &'static str {
    if Provider::uses_responses_api(model) {
        RESPONSES_ENDPOINT
    } else {
        CHAT_ENDPOINT
    }
}

/// Map the unified level to the Responses API `reasoning.effort`. OpenAI has no
/// "max" tier, so High and Max both map to "high"; Off uses "minimal" (gpt-5
/// cannot fully disable reasoning).
fn reasoning_effort(level: ReasoningLevel) -> &'static str {
    match level {
        ReasoningLevel::Off => "minimal",
        ReasoningLevel::Low => "low",
        ReasoningLevel::Medium => "medium",
        ReasoningLevel::High | ReasoningLevel::Max => "high",
    }
}

pub fn build_body(req: &CorrectionRequest) -> Value {
    if is_responses(req) {
        // Responses API: single flattened input + reasoning/verbosity.
        let input = format!(
            "{}\n\n{}",
            req.system_prompt(),
            req.user_message()
        );
        json!({
            "model": req.model,
            "input": input,
            "max_output_tokens": MAX_OUTPUT_TOKENS,
            "reasoning": { "effort": reasoning_effort(req.reasoning_level) },
            "text": { "verbosity": req.verbosity },
            "stream": req.stream,
        })
    } else {
        json!({
            "model": req.model,
            "messages": [
                {"role": "system", "content": req.system_prompt()},
                {"role": "user", "content": req.user_message()},
            ],
            "stream": req.stream,
        })
    }
}

/// Parse a non-streaming response from either API shape.
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    // Chat Completions shape
    if let Some(s) = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
    {
        return Ok(s.to_string());
    }
    // Responses API: top-level convenience field
    if let Some(s) = body.get("output_text").and_then(Value::as_str) {
        if !s.is_empty() {
            return Ok(s.to_string());
        }
    }
    // Responses API: output[].content[].text where type == output_text
    if let Some(items) = body.get("output").and_then(Value::as_array) {
        let mut out = String::new();
        for item in items {
            if let Some(content) = item.get("content").and_then(Value::as_array) {
                for c in content {
                    if c.get("type").and_then(Value::as_str) == Some("output_text") {
                        if let Some(t) = c.get("text").and_then(Value::as_str) {
                            out.push_str(t);
                        }
                    }
                }
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }
    Err(AiError::Response {
        provider: "openai".into(),
        status: None,
        message: "missing content (chat choices / responses output)".into(),
    })
}

/// Parse one SSE line into a content delta, handling both Chat Completions
/// (`choices[].delta.content`) and Responses (`response.output_text.delta`).
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

    // Responses API streaming event
    if v.get("type").and_then(Value::as_str) == Some("response.output_text.delta") {
        return Ok(v
            .get("delta")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()));
    }

    // Chat Completions streaming chunk
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

    fn req(model: &str, stream: bool) -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::OpenAI,
            model: model.into(),
            api_key: "sk-x".into(),
            style: Style::Normal,
            text: "helo".into(),
            stream,
            reasoning_level: ReasoningLevel::High,
            verbosity: "medium".into(),
        }
    }

    #[test]
    fn chat_body_for_standard_model() {
        let b = build_body(&req("gpt-4o-mini", true));
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["stream"], true);
        assert!(b.get("reasoning").is_none());
        assert_eq!(endpoint("gpt-4o-mini"), CHAT_ENDPOINT);
    }

    #[test]
    fn responses_body_for_reasoning_model() {
        let b = build_body(&req("gpt-5-mini", false));
        assert_eq!(b["reasoning"]["effort"], "high");
        assert_eq!(b["text"]["verbosity"], "medium");
        assert!(b["input"].as_str().unwrap().contains("helo"));
        assert_eq!(b["max_output_tokens"], MAX_OUTPUT_TOKENS);
        assert_eq!(endpoint("gpt-5-mini"), RESPONSES_ENDPOINT);
        assert_eq!(endpoint("o1-mini"), RESPONSES_ENDPOINT);
    }

    #[test]
    fn parses_chat_content() {
        let v = json!({"choices":[{"message":{"content":"poprawiony"}}]});
        assert_eq!(parse_response(&v).unwrap(), "poprawiony");
    }

    #[test]
    fn parses_responses_output() {
        let v = json!({"output":[{"type":"message","content":[{"type":"output_text","text":"wynik"}]}]});
        assert_eq!(parse_response(&v).unwrap(), "wynik");
    }

    #[test]
    fn parses_responses_output_text_field() {
        let v = json!({"output_text":"szybki"});
        assert_eq!(parse_response(&v).unwrap(), "szybki");
    }

    #[test]
    fn sse_chat_delta() {
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\"abc\"}}]}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("abc".to_string()));
    }

    #[test]
    fn sse_responses_delta() {
        let line = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"xy\"}";
        assert_eq!(parse_sse_line(line).unwrap(), Some("xy".to_string()));
    }

    #[test]
    fn sse_done_and_empty_are_none() {
        assert_eq!(parse_sse_line("data: [DONE]").unwrap(), None);
        assert_eq!(parse_sse_line(": keep-alive").unwrap(), None);
        assert_eq!(parse_sse_line("event: response.completed").unwrap(), None);
    }

    #[test]
    fn sse_bad_json_errors() {
        assert!(parse_sse_line("data: {not json").is_err());
    }
}
