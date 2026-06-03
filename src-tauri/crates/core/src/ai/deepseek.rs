//! DeepSeek (OpenAI-compatible Chat Completions), parity with
//! `api_clients/deepseek_client.py`. Uses the 35s timeout. Non-streaming MVP.

use serde_json::{json, Value};

use super::error::AiError;
use super::types::CorrectionRequest;

pub const ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
pub const MAX_TOKENS: u32 = 2000;

pub fn build_body(req: &CorrectionRequest) -> Value {
    json!({
        "model": req.model,
        "messages": [
            {"role": "system", "content": req.system_prompt()},
            {"role": "user", "content": req.user_message()},
        ],
        "temperature": 0.7,
        "max_tokens": MAX_TOKENS,
        "stream": req.stream,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::Provider;
    use crate::prompts::Style;

    fn req() -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::DeepSeek,
            model: "deepseek-chat".into(),
            api_key: "sk-deep".into(),
            style: Style::Normal,
            text: "tekst".into(),
            stream: false,
        }
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
