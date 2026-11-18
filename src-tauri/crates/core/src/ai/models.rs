//! A provider's current model names, for the model list in the settings dialog.

use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::error::{provider_detail, AiError};
use super::types::Provider;
use super::{anthropic, gemini};

const TIMEOUT: Duration = Duration::from_secs(15);

/// Fetch the model ids `provider` offers to this key, narrowed to the ones that
/// can correct text and sorted by name.
pub async fn list_models(
    client: &Client,
    provider: Provider,
    api_key: &str,
) -> Result<Vec<String>, AiError> {
    let request = match provider {
        Provider::OpenAI => client.get("https://api.openai.com/v1/models").bearer_auth(api_key),
        Provider::DeepSeek => client.get("https://api.deepseek.com/models").bearer_auth(api_key),
        Provider::Anthropic => client
            .get("https://api.anthropic.com/v1/models?limit=1000")
            .header("x-api-key", api_key)
            .header("anthropic-version", anthropic::API_VERSION),
        Provider::Gemini => client
            .get(format!("{}?pageSize=1000", gemini::BASE))
            .header("x-goog-api-key", api_key),
    };
    let failed = |e: reqwest::Error| AiError::from_reqwest(provider.key(), TIMEOUT.as_secs(), &e);
    let resp = request.timeout(TIMEOUT).send().await.map_err(failed)?;
    let status = resp.status();
    let text = resp.text().await.map_err(failed)?;
    if !status.is_success() {
        return Err(AiError::Response {
            provider: provider.key().into(),
            status: Some(status.as_u16()),
            message: provider_detail(&text),
        });
    }
    let body = serde_json::from_str(&text).map_err(|e| AiError::Response {
        provider: provider.key().into(),
        status: None,
        message: e.to_string(),
    })?;
    Ok(parse_models(provider, &body))
}

/// Words in a model id that mark it as something other than a text model.
const NOT_TEXT: [&str; 12] = [
    "audio", "realtime", "tts", "transcribe", "image", "embedding", "moderation", "search",
    "instruct", "live", "vision", "aqa",
];

/// Model ids from a list response. OpenAI and Gemini also list speech, image
/// and embedding models, which cannot correct text: those are left out.
pub fn parse_models(provider: Provider, body: &Value) -> Vec<String> {
    let items = |key: &str| body[key].as_array().map(Vec::as_slice).unwrap_or_default();
    let text_model = |id: &str| !NOT_TEXT.iter().any(|word| id.contains(word));
    let mut ids: Vec<String> = match provider {
        Provider::Gemini => items("models")
            .iter()
            .filter(|m| {
                m["supportedGenerationMethods"]
                    .as_array()
                    .is_some_and(|methods| methods.iter().any(|x| x == "generateContent"))
            })
            .filter_map(|m| m["name"].as_str())
            .map(|name| name.trim_start_matches("models/"))
            .filter(|id| id.starts_with("gemini-") && text_model(id))
            .map(str::to_owned)
            .collect(),
        _ => items("data")
            .iter()
            .filter_map(|m| m["id"].as_str())
            .filter(|id| match provider {
                Provider::OpenAI => {
                    let chat = id.starts_with("gpt-")
                        || id.starts_with("chatgpt-")
                        // the reasoning families: o1, o3, o4-mini…
                        || (id.starts_with('o') && id[1..].starts_with(|c: char| c.is_ascii_digit()));
                    chat && text_model(id)
                }
                _ => true,
            })
            .map(str::to_owned)
            .collect(),
    };
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_keeps_only_text_models() {
        let body = json!({ "data": [
            { "id": "gpt-5-mini" }, { "id": "o4-mini" }, { "id": "chatgpt-4o-latest" },
            { "id": "gpt-4o-audio-preview" }, { "id": "gpt-4o-mini-tts" }, { "id": "gpt-image-1" },
            { "id": "text-embedding-3-small" }, { "id": "whisper-1" }, { "id": "omni-moderation-latest" },
            { "id": "gpt-5-mini" },
        ]});
        assert_eq!(parse_models(Provider::OpenAI, &body), ["chatgpt-4o-latest", "gpt-5-mini", "o4-mini"]);
    }

    #[test]
    fn gemini_keeps_text_generation_and_drops_the_prefix() {
        let body = json!({ "models": [
            { "name": "models/gemini-2.5-flash", "supportedGenerationMethods": ["generateContent", "countTokens"] },
            { "name": "models/gemini-embedding-001", "supportedGenerationMethods": ["embedContent"] },
            { "name": "models/gemini-2.5-flash-preview-tts", "supportedGenerationMethods": ["generateContent"] },
            { "name": "models/imagen-4.0-generate-001", "supportedGenerationMethods": ["predict"] },
        ]});
        assert_eq!(parse_models(Provider::Gemini, &body), ["gemini-2.5-flash"]);
    }

    #[test]
    fn anthropic_and_deepseek_list_everything_sorted() {
        let body = json!({ "data": [{ "id": "deepseek-reasoner" }, { "id": "deepseek-chat" }] });
        assert_eq!(parse_models(Provider::DeepSeek, &body), ["deepseek-chat", "deepseek-reasoner"]);
        let body = json!({ "data": [{ "id": "claude-sonnet-4-5" }, { "type": "model" }] });
        assert_eq!(parse_models(Provider::Anthropic, &body), ["claude-sonnet-4-5"]);
    }

    #[test]
    fn an_unexpected_body_gives_an_empty_list() {
        assert!(parse_models(Provider::OpenAI, &json!({ "error": "x" })).is_empty());
        assert!(parse_models(Provider::Gemini, &json!([])).is_empty());
    }
}
