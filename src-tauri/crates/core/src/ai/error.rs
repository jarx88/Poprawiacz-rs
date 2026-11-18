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
                // retry on server-side (5xx), rate-limit (429) or unknown status
                status.map(|s| s >= 500 || s == 429).unwrap_or(true)
            }
            AiError::Connection { .. } | AiError::Timeout { .. } => true,
        }
    }

    /// What the panel shows: a Polish sentence saying what went wrong and what to
    /// do about it. `Display` stays technical, for the log.
    /// The frontend offers a "Ustawienia" button when the text says
    /// "w ustawieniach" (`fixableInSettings` in `actions.ts`).
    pub fn user_message(&self) -> String {
        match self {
            AiError::Connection { .. } => {
                "Brak połączenia z dostawcą. Sprawdź internet i spróbuj ponownie.".into()
            }
            AiError::Timeout { seconds, .. } => {
                format!("Dostawca nie odpowiedział w ciągu {seconds} s. Spróbuj ponownie.")
            }
            AiError::Cancelled { .. } => "Anulowano.".into(),
            AiError::Response { status, message, .. } => {
                let lead = match status {
                    Some(401 | 403) => "Dostawca odrzucił klucz API. Sprawdź klucz w ustawieniach.",
                    Some(404) => "Dostawca nie zna tego modelu. Sprawdź nazwę modelu w ustawieniach.",
                    Some(429 | 402) => "Limit zapytań albo środków u dostawcy jest wyczerpany. Spróbuj za chwilę.",
                    Some(400 | 422) => "Dostawca odrzucił zapytanie. Sprawdź model i klucz w ustawieniach.",
                    Some(s) if *s >= 500 => "Serwer dostawcy ma awarię albo jest przeciążony. Spróbuj za chwilę.",
                    Some(_) => "Dostawca zwrócił błąd.",
                    None => "Dostawca zwrócił odpowiedź, której nie da się odczytać.",
                };
                let detail = provider_detail(message);
                match (status, detail.is_empty()) {
                    (Some(s), true) => format!("{lead} (HTTP {s})"),
                    (Some(s), false) => format!("{lead} Szczegóły (HTTP {s}): {detail}"),
                    (None, true) => lead.into(),
                    (None, false) => format!("{lead} Szczegóły: {detail}"),
                }
            }
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

/// The provider's own explanation: `error.message` from a JSON error body (an
/// array of such objects for Gemini streams), otherwise the body as it came,
/// cut to a length that fits a panel. Applied where the body is read, so the
/// error (and the log) never carries a whole raw response.
pub(crate) fn provider_detail(body: &str) -> String {
    let text = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.pointer("/0/error/message"))
                .and_then(|m| m.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| body.trim().to_owned());
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: Option<u16>, message: &str) -> String {
        AiError::Response { provider: "x".into(), status, message: message.into() }.user_message()
    }

    #[test]
    fn user_message_names_the_cause_and_the_fix() {
        assert_eq!(
            response(Some(401), r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#),
            "Dostawca odrzucił klucz API. Sprawdź klucz w ustawieniach. Szczegóły (HTTP 401): invalid x-api-key"
        );
        assert!(response(Some(404), "").starts_with("Dostawca nie zna tego modelu."));
        assert!(response(Some(404), "").ends_with("(HTTP 404)"));
        assert!(response(Some(429), "").starts_with("Limit zapytań"));
        assert!(response(Some(503), "").starts_with("Serwer dostawcy"));
        assert_eq!(
            AiError::Timeout { provider: "x".into(), seconds: 25 }.user_message(),
            "Dostawca nie odpowiedział w ciągu 25 s. Spróbuj ponownie."
        );
    }

    #[test]
    fn user_message_detail_survives_odd_bodies() {
        // Gemini stream errors arrive as an array
        assert!(response(Some(400), r#"[{"error":{"message":"bad model"}}]"#).ends_with("(HTTP 400): bad model"));
        // a body cut mid-JSON, or not JSON at all, is shown as it came
        assert!(response(Some(500), "<html>Bad Gateway").ends_with("(HTTP 500): <html>Bad Gateway"));
        assert_eq!(
            response(None, "empty candidate text"),
            "Dostawca zwrócił odpowiedź, której nie da się odczytać. Szczegóły: empty candidate text"
        );
        // long bodies are cut on a character boundary
        assert!(response(Some(500), &"ż".repeat(900)).chars().count() < 300);
    }

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
