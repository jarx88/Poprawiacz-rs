//! App-side modules: native OS integration, secrets, and command boundary.
//! Pure logic lives in the `poprawiacz-core` crate.

pub mod ai;
pub mod clipboard;
pub mod config;
pub mod hotkey;
pub mod logging;

use std::sync::atomic::AtomicU64;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Shared application state held by Tauri.
pub struct AppState {
    /// Reused HTTP client (connection pooling) for all providers.
    pub http: reqwest::Client,
    /// Monotonic correction-session counter.
    pub session: AtomicU64,
    /// Cancellation token of the in-flight session (cancels the previous one).
    pub cancel: Mutex<Option<CancellationToken>>,
}

impl AppState {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            session: AtomicU64::new(0),
            cancel: Mutex::new(None),
        }
    }
}
