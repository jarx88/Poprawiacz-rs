//! App-side modules: native OS integration, secrets, and command boundary.
//! Pure logic lives in the `poprawiacz-core` crate.

pub mod ai;
pub mod autostart;
pub mod clipboard;
pub mod config;
pub mod hotkey;
pub mod logging;
pub mod tray;

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
}

impl AppState {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            session: AtomicU64::new(0),
            cancel: Mutex::new(SessionCancels::default()),
            last_input: Mutex::new(None),
        }
    }
}
