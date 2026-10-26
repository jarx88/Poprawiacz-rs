//! AI provider orchestration: request building, HTTP execution with timeouts,
//! retry/backoff, cancellation, and OpenAI streaming. All provider-specific
//! request/response shaping lives in the per-provider submodules and is unit
//! tested without the network.

pub mod anthropic;
pub mod deepseek;
pub mod error;
pub mod gemini;
pub mod openai;
pub mod retry;
pub mod types;

pub use error::AiError;
pub use types::{
    CorrectionRequest, Provider, ReasoningLevel, CONNECT_TIMEOUT, DEEPSEEK_TIMEOUT, MAX_RETRIES,
    STANDARD_TIMEOUT,
};

use futures_util::StreamExt;
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Fixed backoff between retries.
pub const RETRY_BACKOFF: Duration = Duration::from_millis(800);

/// Build a shared HTTP client with the common connect timeout.
pub fn build_client() -> Result<Client, reqwest::Error> {
    Client::builder().connect_timeout(CONNECT_TIMEOUT).build()
}

fn build_request(client: &Client, req: &CorrectionRequest) -> RequestBuilder {
    match req.provider {
        Provider::OpenAI => client
            .post(openai::endpoint(&req.model))
            .bearer_auth(&req.api_key)
            .json(&openai::build_body(req)),
        Provider::DeepSeek => client
            .post(deepseek::ENDPOINT)
            .bearer_auth(&req.api_key)
            .json(&deepseek::build_body(req)),
        Provider::Anthropic => client
            .post(anthropic::ENDPOINT)
            .header("x-api-key", &req.api_key)
            .header("anthropic-version", anthropic::API_VERSION)
            .json(&anthropic::build_body(req)),
        Provider::Gemini => {
            let url = if req.stream {
                gemini::stream_endpoint(&req.model)
            } else {
                gemini::endpoint(&req.model)
            };
            client
                .post(url)
                .header("x-goog-api-key", &req.api_key)
                .json(&gemini::build_body(req))
        }
    }
}

fn parse_body(provider: Provider, body: &Value) -> Result<String, AiError> {
    match provider {
        Provider::OpenAI => openai::parse_response(body),
        Provider::Anthropic => anthropic::parse_response(body),
        Provider::Gemini => gemini::parse_response(body),
        Provider::DeepSeek => deepseek::parse_response(body),
    }
}

/// Dispatch one streamed SSE line to the provider's delta parser.
fn parse_stream_line(provider: Provider, line: &str) -> Result<Option<String>, AiError> {
    match provider {
        Provider::OpenAI => openai::parse_sse_line(line),
        Provider::Anthropic => anthropic::parse_sse_line(line),
        Provider::Gemini => gemini::parse_sse_line(line),
        Provider::DeepSeek => deepseek::parse_sse_line(line),
    }
}

/// Whether an SSE line signals end-of-stream. Critical: OpenAI/DeepSeek send
/// `data: [DONE]` but keep the HTTP connection alive, so we must stop on the
/// sentinel instead of waiting for the body to close (which would hang).
fn is_stream_done(provider: Provider, line: &str) -> bool {
    let l = line.trim();
    match provider {
        Provider::OpenAI | Provider::DeepSeek => l == "data: [DONE]" || l == "[DONE]",
        Provider::Anthropic => l.starts_with("data:") && l.contains("\"type\":\"message_stop\""),
        Provider::Gemini => false,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

async fn send_once(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
) -> Result<String, AiError> {
    let provider = req.provider;
    let timeout = provider.timeout();
    let request = build_request(client, req).timeout(timeout);

    let resp = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AiError::Cancelled { provider: provider.key().into() }),
        r = request.send() => r.map_err(|e| AiError::from_reqwest(provider.key(), timeout.as_secs(), &e))?,
    };

    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        return Err(AiError::Response {
            provider: provider.key().into(),
            status: Some(status.as_u16()),
            message: truncate(&txt, 500),
        });
    }

    // reqwest's request timeout only covers connect+headers; bound the body
    // read explicitly so a slow/stalled body can't hang forever.
    let body: Value = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AiError::Cancelled { provider: provider.key().into() }),
        r = tokio::time::timeout(timeout, resp.json::<Value>()) => match r {
            Err(_) => return Err(AiError::Timeout { provider: provider.key().into(), seconds: timeout.as_secs() }),
            Ok(inner) => inner.map_err(|e| AiError::from_reqwest(provider.key(), timeout.as_secs(), &e))?,
        },
    };

    parse_body(provider, &body).map(|t| strip_ai_dashes(&t))
}

/// Models sneak em/en dashes into output despite the prompt ban — a dead
/// giveaway of AI text. Deterministically replace them with a plain hyphen.
fn strip_ai_dashes(s: &str) -> String {
    if s.contains('\u{2014}') || s.contains('\u{2013}') {
        s.replace(['\u{2014}', '\u{2013}'], "-")
    } else {
        s.to_string()
    }
}

/// Run one correction (non-streaming) with retry, timeout and cancellation.
pub async fn correct(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
) -> Result<String, AiError> {
    let fallback = fallback_request(req);
    // Retrying an overloaded Gemini model only adds delay; go to the fallback.
    let retries = if fallback.is_some() { 0 } else { MAX_RETRIES };
    let result = retry::with_retries(retries, RETRY_BACKOFF, |_attempt| send_once(client, req, cancel)).await;
    match (result, fallback) {
        (Err(e), Some(fb)) if e.is_retryable() => {
            tracing::warn!(model = %req.model, fallback = %fb.model, "gemini: {e}; trying fallback model");
            retry::with_retries(MAX_RETRIES, RETRY_BACKOFF, |_attempt| send_once(client, &fb, cancel)).await
        }
        (result, _) => result,
    }
}

/// Gemini only: the same request on [`gemini::FALLBACK_MODEL`], used when the
/// configured model is overloaded (503), out of free-tier quota (429) or
/// times out.
fn fallback_request(req: &CorrectionRequest) -> Option<CorrectionRequest> {
    if req.provider != Provider::Gemini {
        return None;
    }
    gemini::fallback_model(&req.model).map(|model| CorrectionRequest { model: model.into(), ..req.clone() })
}

/// Run a streaming correction (any provider). `on_chunk` is called for every
/// content delta; the full accumulated text is returned. Retries (like the
/// non-streaming path) as long as NO chunk has been emitted yet — a 503/429
/// before the first delta is safe to re-run; a half-stream is not.
pub async fn correct_stream<F>(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
    mut on_chunk: F,
) -> Result<String, AiError>
where
    F: FnMut(&str),
{
    let fallback = fallback_request(req);
    let retries = if fallback.is_some() { 0 } else { MAX_RETRIES };
    let (result, emitted) = stream_with_retries(client, req, cancel, retries, &mut on_chunk).await;
    match (result, fallback) {
        (Err(e), Some(fb)) if !emitted && e.is_retryable() => {
            tracing::warn!(model = %req.model, fallback = %fb.model, "gemini: {e}; trying fallback model");
            stream_with_retries(client, &fb, cancel, MAX_RETRIES, &mut on_chunk).await.0
        }
        (result, _) => result,
    }
}

/// Streaming attempts of one request; also reports whether any chunk reached
/// `on_chunk` (then neither a retry nor a fallback is safe).
async fn stream_with_retries<F: FnMut(&str)>(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
    max_retries: u32,
    on_chunk: &mut F,
) -> (Result<String, AiError>, bool) {
    let mut attempt = 0u32;
    loop {
        let mut emitted = false;
        let result = stream_once(client, req, cancel, |delta: &str| {
            emitted = true;
            on_chunk(delta);
        })
        .await;
        match result {
            Ok(v) => return (Ok(v), emitted),
            Err(e) => {
                if emitted || attempt >= max_retries || !e.is_retryable() {
                    return (Err(e), emitted);
                }
                attempt += 1;
                tokio::time::sleep(RETRY_BACKOFF * 2u32.saturating_pow(attempt - 1)).await;
            }
        }
    }
}

async fn stream_once<F: FnMut(&str)>(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
    mut on_chunk: F,
) -> Result<String, AiError> {
    debug_assert!(req.stream);
    let provider = req.provider;
    let timeout = provider.timeout();
    let request = build_request(client, req).timeout(timeout);

    let resp = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AiError::Cancelled { provider: provider.key().into() }),
        r = request.send() => r.map_err(|e| AiError::from_reqwest(provider.key(), timeout.as_secs(), &e))?,
    };

    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        return Err(AiError::Response {
            provider: provider.key().into(),
            status: Some(status.as_u16()),
            message: truncate(&txt, 500),
        });
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut full = String::new();

    loop {
        // Idle timeout: abort if no data arrives within the provider timeout
        // (reqwest's request timeout does not cover streamed body reads).
        let next = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AiError::Cancelled { provider: provider.key().into() }),
            r = tokio::time::timeout(timeout, stream.next()) => match r {
                Err(_) => return Err(AiError::Timeout { provider: provider.key().into(), seconds: timeout.as_secs() }),
                Ok(n) => n,
            },
        };
        let Some(chunk) = next else { break };
        let bytes = chunk.map_err(|e| AiError::from_reqwest(provider.key(), timeout.as_secs(), &e))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete lines; keep the trailing partial line in `buf`.
        let mut done = false;
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            if is_stream_done(provider, &line) {
                done = true;
                break;
            }
            if let Some(delta) = parse_stream_line(provider, &line)? {
                let delta = strip_ai_dashes(&delta);
                full.push_str(&delta);
                on_chunk(&delta);
            }
        }
        if done {
            break; // stop on [DONE]/message_stop instead of waiting for close
        }
    }
    // flush any trailing line without newline
    if !buf.trim().is_empty() {
        if let Some(delta) = parse_stream_line(provider, &buf)? {
            let delta = strip_ai_dashes(&delta);
            full.push_str(&delta);
            on_chunk(&delta);
        }
    }

    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ai_dashes_replaces_em_and_en_dash() {
        assert_eq!(strip_ai_dashes("tekst — wtrącenie – koniec"), "tekst - wtrącenie - koniec");
        assert_eq!(strip_ai_dashes("zwykły tekst"), "zwykły tekst");
    }

    #[test]
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }

    #[test]
    fn only_gemini_falls_back_and_never_to_itself() {
        let mut req = CorrectionRequest {
            provider: Provider::Gemini,
            model: "gemini-3.1-flash-lite".into(),
            api_key: "k".into(),
            style: crate::Style::Normal,
            text: "t".into(),
            stream: true,
            reasoning_level: ReasoningLevel::Off,
            verbosity: "medium".into(),
            custom_system_prompt: None,
            correction_examples: Vec::new(),
        };
        let fb = fallback_request(&req).unwrap();
        assert_eq!(fb.model, gemini::FALLBACK_MODEL);
        assert_eq!(fb.text, req.text);
        req.model = gemini::FALLBACK_MODEL.into();
        assert!(fallback_request(&req).is_none());
        req.provider = Provider::OpenAI;
        req.model = "gpt-5-mini".into();
        assert!(fallback_request(&req).is_none());
    }

    #[test]
    fn client_builds() {
        assert!(build_client().is_ok());
    }

    #[test]
    fn stream_done_detection() {
        assert!(is_stream_done(Provider::OpenAI, "data: [DONE]"));
        assert!(is_stream_done(Provider::DeepSeek, "data: [DONE]"));
        assert!(!is_stream_done(Provider::OpenAI, "data: {\"choices\":[]}"));
        assert!(is_stream_done(
            Provider::Anthropic,
            "data: {\"type\":\"message_stop\"}"
        ));
        assert!(!is_stream_done(Provider::Anthropic, "data: {\"type\":\"ping\"}"));
        assert!(!is_stream_done(Provider::Gemini, "data: {}"));
    }
}
