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
    CorrectionRequest, Provider, CONNECT_TIMEOUT, DEEPSEEK_TIMEOUT, MAX_RETRIES, STANDARD_TIMEOUT,
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

    parse_body(provider, &body)
}

/// Run one correction (non-streaming) with retry, timeout and cancellation.
pub async fn correct(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
) -> Result<String, AiError> {
    retry::with_retries(MAX_RETRIES, RETRY_BACKOFF, |_attempt| {
        send_once(client, req, cancel)
    })
    .await
}

/// Run a streaming correction (any provider). `on_chunk` is called for every
/// content delta; the full accumulated text is returned. No retry on a
/// partially streamed response (a half-stream cannot be safely re-run).
pub async fn correct_stream<F>(
    client: &Client,
    req: &CorrectionRequest,
    cancel: &CancellationToken,
    mut on_chunk: F,
) -> Result<String, AiError>
where
    F: FnMut(&str),
{
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
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            if let Some(delta) = parse_stream_line(provider, &line)? {
                full.push_str(&delta);
                on_chunk(&delta);
            }
        }
    }
    // flush any trailing line without newline
    if !buf.trim().is_empty() {
        if let Some(delta) = parse_stream_line(provider, &buf)? {
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
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }

    #[test]
    fn client_builds() {
        assert!(build_client().is_ok());
    }
}
