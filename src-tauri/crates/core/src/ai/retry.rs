//! Retry/backoff, parity with `DEFAULT_RETRIES = 2` and a fixed backoff.
//!
//! Non-retryable errors (cancellation, 4xx) short-circuit immediately.

use std::future::Future;
use std::time::Duration;

use super::error::AiError;

/// Run `op` up to `max_retries + 1` times. `op` receives the zero-based attempt
/// index. Retries only on retryable errors, sleeping `backoff` between attempts.
pub async fn with_retries<T, F, Fut>(
    max_retries: u32,
    backoff: Duration,
    mut op: F,
) -> Result<T, AiError>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, AiError>>,
{
    let mut attempt = 0u32;
    loop {
        match op(attempt).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let can_retry = attempt < max_retries && e.is_retryable();
                if !can_retry {
                    return Err(e);
                }
                attempt += 1;
                if !backoff.is_zero() {
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[tokio::test]
    async fn succeeds_first_try() {
        let calls = Cell::new(0);
        let r: Result<i32, AiError> = with_retries(2, Duration::ZERO, |_| {
            calls.set(calls.get() + 1);
            async { Ok(42) }
        })
        .await;
        assert_eq!(r.unwrap(), 42);
        assert_eq!(calls.get(), 1);
    }

    #[tokio::test]
    async fn retries_retryable_then_succeeds() {
        let calls = Cell::new(0);
        let r: Result<i32, AiError> = with_retries(2, Duration::ZERO, |attempt| {
            calls.set(calls.get() + 1);
            async move {
                if attempt < 2 {
                    Err(AiError::Timeout { provider: "x".into(), seconds: 25 })
                } else {
                    Ok(7)
                }
            }
        })
        .await;
        assert_eq!(r.unwrap(), 7);
        assert_eq!(calls.get(), 3); // initial + 2 retries
    }

    #[tokio::test]
    async fn exhausts_retries_and_returns_last_error() {
        let calls = Cell::new(0);
        let r: Result<i32, AiError> = with_retries(2, Duration::ZERO, |_| {
            calls.set(calls.get() + 1);
            async { Err(AiError::Connection { provider: "x".into(), message: "m".into() }) }
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.get(), 3); // initial + 2 retries, then give up
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable() {
        let calls = Cell::new(0);
        let r: Result<i32, AiError> = with_retries(2, Duration::ZERO, |_| {
            calls.set(calls.get() + 1);
            async { Err(AiError::Cancelled { provider: "x".into() }) }
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.get(), 1); // no retries
    }
}
