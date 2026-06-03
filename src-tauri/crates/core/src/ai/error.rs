//! AI error types, parity with the Python custom exceptions
//! (`APIConnectionError`, `APIResponseError`, `APITimeoutError`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    /// Network/connection failure (DNS, TLS, refused).
    #[error("connection error ({provider}): {message}")]
    Connection { provider: String, message: String },

    /// Non-2xx HTTP response or unparseable body.
    #[error("response error ({provider}, status {status:?}): {message}")]
    Response {
        provider: String,
        status: Option<u16>,
        message: String,
    },

    /// Read/connect timed out.
    #[error("timeout ({provider}) after {seconds}s")]
    Timeout { provider: String, seconds: u64 },

    /// The session was cancelled before completion (newer session started).
    #[error("cancelled ({provider})")]
    Cancelled { provider: String },
}

impl AiError {
    /// Whether a retry could plausibly succeed. Cancellation and client (4xx)
    /// errors are not retried; connection/timeout/5xx are.
    pub fn is_retryable(&self) -> bool {
        match self {
            AiError::Cancelled { .. } => false,
            AiError::Response { status, .. } => {
                // retry only on server-side (5xx) or unknown status
                status.map(|s| s >= 500).unwrap_or(true)
            }
            AiError::Connection { .. } | AiError::Timeout { .. } => true,
        }
    }

    /// Classify a `reqwest::Error` into an `AiError` for a provider.
    pub fn from_reqwest(provider: &str, seconds: u64, err: &reqwest::Error) -> AiError {
        if err.is_timeout() {
            AiError::Timeout {
                provider: provider.to_string(),
                seconds,
            }
        } else if err.is_connect() {
            AiError::Connection {
                provider: provider.to_string(),
                message: err.to_string(),
            }
        } else {
            AiError::Response {
                provider: provider.to_string(),
                status: err.status().map(|s| s.as_u16()),
                message: err.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_not_retryable() {
        let e = AiError::Cancelled { provider: "openai".into() };
        assert!(!e.is_retryable());
    }

    #[test]
    fn connection_and_timeout_are_retryable() {
        assert!(AiError::Connection { provider: "x".into(), message: "m".into() }.is_retryable());
        assert!(AiError::Timeout { provider: "x".into(), seconds: 25 }.is_retryable());
    }

    #[test]
    fn client_errors_not_retried_server_errors_are() {
        assert!(!AiError::Response { provider: "x".into(), status: Some(401), message: "m".into() }.is_retryable());
        assert!(!AiError::Response { provider: "x".into(), status: Some(400), message: "m".into() }.is_retryable());
        assert!(AiError::Response { provider: "x".into(), status: Some(503), message: "m".into() }.is_retryable());
        assert!(AiError::Response { provider: "x".into(), status: None, message: "m".into() }.is_retryable());
    }
}
