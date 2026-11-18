//! OpenRouter: one key and one OpenAI-compatible endpoint for every panel.
//! A panel keeps its provider identity; only the route and the model id
//! (`anthropic/claude-…`) differ. Shapes verified live (Nov 2026).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};

use super::deepseek;
use super::error::{provider_detail, AiError};
use super::types::{CorrectionRequest, Provider, ReasoningLevel};
use crate::config::ProviderModels;

pub const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
/// Public: needs no key.
pub const MODELS_ENDPOINT: &str = "https://openrouter.ai/api/v1/models";
const MODELS_TIMEOUT: Duration = Duration::from_secs(15);

/// The vendor prefix of the models a panel offers.
fn vendor(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "openai/",
        Provider::Anthropic => "anthropic/",
        Provider::Gemini => "google/",
        Provider::DeepSeek => "deepseek/",
    }
}

/// A fast, cheap model per panel, used until the user picks one.
pub fn default_model(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "openai/gpt-5-mini",
        Provider::Anthropic => "anthropic/claude-haiku-4.5",
        Provider::Gemini => "google/gemini-3.6-flash",
        Provider::DeepSeek => "deepseek/deepseek-v4-flash",
    }
}

/// The panel's OpenRouter model, falling back to [`default_model`].
pub fn resolved_model(models: &ProviderModels, provider: Provider) -> String {
    models
        .chosen(provider)
        .map_or_else(|| default_model(provider).to_string(), str::to_owned)
}

/// Models that answered `effort: "none"` with "Reasoning is mandatory…" during
/// this run. For them Off means "minimal", the least they accept; for the rest
/// "none" is faster (no reasoning tokens at all).
static MANDATORY_REASONING: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn reasoning_is_mandatory(model: &str) -> bool {
    MANDATORY_REASONING
        .lock()
        .expect("reasoning memo poisoned")
        .iter()
        .any(|m| m == model)
}

/// True when `err` is OpenRouter refusing to switch reasoning off for this
/// model. The model is remembered, so the caller's one repeat of the request
/// (and every later one) asks for "minimal". It answers by what this request
/// was refused for, not by the memo: two panels may hit the refusal at once.
pub fn learn_mandatory_reasoning(req: &CorrectionRequest, err: &AiError) -> bool {
    let refused = req.openrouter
        && req.reasoning_level == ReasoningLevel::Off
        && matches!(err, AiError::Response { status: Some(400), message, .. }
            if message.contains("Reasoning is mandatory"));
    if refused {
        let mut known = MANDATORY_REASONING.lock().expect("reasoning memo poisoned");
        if !known.contains(&req.model) {
            known.push(req.model.clone());
        }
    }
    refused
}

fn effort(req: &CorrectionRequest) -> &'static str {
    match req.reasoning_level {
        ReasoningLevel::Off if reasoning_is_mandatory(&req.model) => "minimal",
        ReasoningLevel::Off => "none",
        ReasoningLevel::Low => "low",
        ReasoningLevel::Medium => "medium",
        ReasoningLevel::High | ReasoningLevel::Max => "high",
    }
}

pub fn build_body(req: &CorrectionRequest) -> Value {
    json!({
        "model": req.model,
        "messages": [
            {"role": "system", "content": req.system_prompt()},
            {"role": "user", "content": req.user_message()},
        ],
        "stream": req.stream,
        // `exclude`: the reasoning text itself is never sent back
        "reasoning": { "effort": effort(req), "exclude": true },
    })
}

/// OpenRouter reports some failures inside a 200 response or mid-stream.
fn embedded_error(body: &Value) -> Option<AiError> {
    let message = body.pointer("/error/message")?.as_str()?;
    Some(AiError::Response {
        provider: "openrouter".into(),
        // the code is a number in live answers; a string is accepted too
        status: body
            .pointer("/error/code")
            .and_then(|c| c.as_u64().or_else(|| c.as_str()?.parse().ok()))
            .and_then(|c| u16::try_from(c).ok()),
        message: message.chars().take(200).collect(),
    })
}

/// Parse: `choices[0].message.content`, or the error OpenRouter put in the body.
pub fn parse_response(body: &Value) -> Result<String, AiError> {
    match embedded_error(body) {
        Some(e) => Err(e),
        None => deepseek::parse_response(body),
    }
}

/// Parse one SSE line: a content delta, or an error chunk. Comment lines
/// (`: OPENROUTER PROCESSING`) and `[DONE]` carry nothing.
pub fn parse_sse_line(line: &str) -> Result<Option<String>, AiError> {
    let payload = line.trim().strip_prefix("data:").map(str::trim);
    if let Some(e) = payload
        .and_then(|p| serde_json::from_str::<Value>(p).ok())
        .and_then(|v| embedded_error(&v))
    {
        return Err(e);
    }
    deepseek::parse_sse_line(line)
}

/// The text models OpenRouter offers for each panel, sorted by name.
pub async fn list_models(client: &Client) -> Result<HashMap<Provider, Vec<String>>, AiError> {
    let failed = |e: reqwest::Error| AiError::from_reqwest("openrouter", MODELS_TIMEOUT.as_secs(), &e);
    let resp = client.get(MODELS_ENDPOINT).timeout(MODELS_TIMEOUT).send().await.map_err(failed)?;
    let status = resp.status();
    let text = resp.text().await.map_err(failed)?;
    if !status.is_success() {
        return Err(AiError::Response {
            provider: "openrouter".into(),
            status: Some(status.as_u16()),
            message: provider_detail(&text),
        });
    }
    let body: Value = serde_json::from_str(&text).map_err(|e| AiError::Response {
        provider: "openrouter".into(),
        status: None,
        message: e.to_string(),
    })?;
    Ok(Provider::ALL.into_iter().map(|p| (p, parse_models(p, &body))).collect())
}

/// Ids of `provider`'s vendor that answer with text only. `:batch` variants are
/// left out: they are not meant for an interactive request.
pub fn parse_models(provider: Provider, body: &Value) -> Vec<String> {
    let mut ids: Vec<String> = body["data"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|m| m.pointer("/architecture/output_modalities") == Some(&json!(["text"])))
        .filter_map(|m| m["id"].as_str())
        .filter(|id| id.starts_with(vendor(provider)) && !id.ends_with(":batch"))
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::Style;

    fn req(model: &str, level: ReasoningLevel) -> CorrectionRequest {
        CorrectionRequest {
            provider: Provider::Anthropic,
            model: model.into(),
            api_key: "sk-or-x".into(),
            style: Style::Normal,
            text: "tekst".into(),
            stream: true,
            reasoning_level: level,
            verbosity: "medium".into(),
            custom_system_prompt: None,
            correction_examples: Vec::new(),
            openrouter: true,
        }
    }

    fn mandatory() -> AiError {
        AiError::Response {
            provider: "anthropic".into(),
            status: Some(400),
            message: "Reasoning is mandatory for this endpoint and cannot be disabled.".into(),
        }
    }

    #[test]
    fn body_is_openai_compatible_and_maps_the_reasoning_level() {
        let body = build_body(&req("anthropic/test-plain", ReasoningLevel::Off));
        assert_eq!(body["model"], "anthropic/test-plain");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["reasoning"], json!({ "effort": "none", "exclude": true }));
        let high = build_body(&req("anthropic/test-plain", ReasoningLevel::Max));
        assert_eq!(high["reasoning"]["effort"], "high");
    }

    #[test]
    fn a_model_that_needs_reasoning_is_learned_and_then_asked_for_minimal() {
        let off = req("anthropic/test-mandatory", ReasoningLevel::Off);
        assert_eq!(build_body(&off)["reasoning"]["effort"], "none");
        assert!(learn_mandatory_reasoning(&off, &mandatory()));
        assert_eq!(build_body(&off)["reasoning"]["effort"], "minimal");
        // a second panel refused at the same moment repeats its request too
        assert!(learn_mandatory_reasoning(&off, &mandatory()));
        // other levels, other errors and the direct route never trigger it
        assert!(!learn_mandatory_reasoning(&req("anthropic/test-low", ReasoningLevel::Low), &mandatory()));
        let timeout = AiError::Timeout { provider: "x".into(), seconds: 25 };
        assert!(!learn_mandatory_reasoning(&req("anthropic/test-other", ReasoningLevel::Off), &timeout));
        let direct = CorrectionRequest { openrouter: false, ..req("anthropic/test-direct", ReasoningLevel::Off) };
        assert!(!learn_mandatory_reasoning(&direct, &mandatory()));
    }

    #[test]
    fn errors_inside_a_response_or_a_stream_are_reported() {
        let body = json!({ "error": { "message": "openai/x is not a valid model ID", "code": 400 } });
        let err = parse_response(&body).unwrap_err();
        assert!(matches!(err, AiError::Response { status: Some(400), .. }));
        let funds = parse_response(&json!({ "error": { "message": "no credits", "code": "402" } }));
        assert!(matches!(funds, Err(AiError::Response { status: Some(402), .. })));
        assert!(parse_sse_line(r#"data: {"error":{"message":"Provider returned error","code":502}}"#).is_err());
    }

    #[test]
    fn stream_lines_yield_content_only() {
        assert_eq!(parse_sse_line(": OPENROUTER PROCESSING").unwrap(), None);
        assert_eq!(parse_sse_line("data: [DONE]").unwrap(), None);
        let line = r#"data: {"choices":[{"index":0,"delta":{"role":"assistant","content":"Ala"}}]}"#;
        assert_eq!(parse_sse_line(line).unwrap().as_deref(), Some("Ala"));
    }

    #[test]
    fn models_are_split_by_vendor_and_limited_to_text() {
        let text = json!({ "output_modalities": ["text"] });
        let body = json!({ "data": [
            { "id": "openai/gpt-5-mini", "architecture": text },
            { "id": "openai/gpt-5-mini:batch", "architecture": text },
            { "id": "openai/gpt-5.4-image-2", "architecture": { "output_modalities": ["image", "text"] } },
            { "id": "google/gemini-3.6-flash", "architecture": text },
            { "id": "anthropic/claude-haiku-4.5", "architecture": text },
            { "id": "mistralai/mistral-large", "architecture": text },
        ]});
        assert_eq!(parse_models(Provider::OpenAI, &body), ["openai/gpt-5-mini"]);
        assert_eq!(parse_models(Provider::Gemini, &body), ["google/gemini-3.6-flash"]);
        assert!(parse_models(Provider::DeepSeek, &body).is_empty());
    }

    #[test]
    fn an_unset_model_falls_back_to_the_default() {
        let models = ProviderModels { anthropic: Some("anthropic/claude-sonnet-5".into()), ..Default::default() };
        assert_eq!(resolved_model(&models, Provider::Anthropic), "anthropic/claude-sonnet-5");
        assert_eq!(resolved_model(&models, Provider::OpenAI), "openai/gpt-5-mini");
    }
}
