//! App-side modules: native OS integration, secrets, and command boundary.
//! Pure logic lives in the `poprawiacz-core` crate.

pub mod ai;
pub mod autostart;
pub mod clipboard;
pub mod config;
pub mod hotkey;
pub mod logging;
pub mod tray;
pub mod updater;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;
use poprawiacz_core::ai::Provider;
use poprawiacz_core::Style;
use tokio_util::sync::CancellationToken;

/// Per-provider cancellation tokens for the in-flight session.
#[derive(Default)]
pub struct SessionCancels {
    pub tokens: HashMap<Provider, CancellationToken>,
}

impl SessionCancels {
    /// Cancel every in-flight provider task and clear the map.
    pub fn cancel_all(&mut self) {
        for (_, t) in self.tokens.drain() {
            t.cancel();
        }
    }
    /// Cancel a single provider's task.
    pub fn cancel_one(&self, provider: Provider) {
        if let Some(t) = self.tokens.get(&provider) {
            t.cancel();
        }
    }
    /// True when `token` was cancelled to make room for a newer attempt
    /// (reprocess, new session): whatever its task still reports is stale.
    /// A plain cancel leaves no live token behind, so it is not superseded.
    pub fn superseded(&self, provider: Provider, token: &CancellationToken) -> bool {
        token.is_cancelled() && self.tokens.get(&provider).is_some_and(|t| !t.is_cancelled())
    }
}

/// Shared application state held by Tauri.
pub struct AppState {
    /// Reused HTTP client (connection pooling) for all providers.
    pub http: reqwest::Client,
    /// Monotonic correction-session counter.
    pub session: AtomicU64,
    /// Per-provider cancellation tokens of the in-flight session.
    pub cancel: Mutex<SessionCancels>,
    /// Last correction input: (session_id, text, style) — for single-panel reprocess.
    pub last_input: Mutex<Option<(u64, String, Style)>>,
    /// Unix seconds of the last correction start; the updater waits for quiet.
    pub last_activity_s: AtomicU64,
}

impl AppState {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            session: AtomicU64::new(0),
            cancel: Mutex::new(SessionCancels::default()),
            last_input: Mutex::new(None),
            last_activity_s: AtomicU64::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_replaced_attempt_is_superseded() {
        let p = Provider::OpenAI;
        let mut cancels = SessionCancels::default();
        let first = CancellationToken::new();
        cancels.tokens.insert(p, first.clone());
        assert!(!cancels.superseded(p, &first)); // running

        cancels.cancel_one(p);
        assert!(!cancels.superseded(p, &first)); // cancelled by the user: must report

        cancels.tokens.insert(p, CancellationToken::new()); // reprocess
        assert!(cancels.superseded(p, &first));

        let second = cancels.tokens[&p].clone();
        cancels.cancel_all(); // "Anuluj wszystko", closing the window
        assert!(!cancels.superseded(p, &second));
    }
}
