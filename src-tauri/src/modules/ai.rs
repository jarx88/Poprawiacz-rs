//! Correction orchestration: fan out to all providers concurrently, emit
//! per-provider streaming/result/error events tagged with the session id.

use std::time::Instant;

use poprawiacz_core::ai::{self, AiError, CorrectionRequest, Provider};
use poprawiacz_core::Style;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use super::config;
use super::AppState;

/// Start a new correction session for `text`. Cancels the previous session,
/// then spawns one task per provider. Returns the new session id.
pub fn run_correction(app: &AppHandle, text: String, style: Style) -> u64 {
    let state = app.state::<AppState>();
    let session_id = state
        .session
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;

    // New cancellation token; cancel the previous session's in-flight tasks.
    let token = CancellationToken::new();
    {
        let mut guard = state.cancel.lock().expect("cancel mutex poisoned");
        if let Some(old) = guard.take() {
            old.cancel();
        }
        *guard = Some(token.clone());
    }

    let _ = app.emit(
        "session-started",
        json!({ "session_id": session_id, "text": text }),
    );

    let settings = config::load_settings(app);
    let http = state.http.clone();

    for provider in Provider::ALL {
        let app = app.clone();
        let http = http.clone();
        let token = token.clone();
        let text = text.clone();
        let model = settings.models.resolved(provider);
        let key = config::get_api_key(provider);

        tauri::async_runtime::spawn(async move {
            let Some(api_key) = key else {
                emit_error(&app, session_id, provider, "Brak klucza API");
                return;
            };

            let stream = provider.supports_streaming();
            let req = CorrectionRequest {
                provider,
                model,
                api_key,
                style,
                text,
                stream,
            };

            let started = Instant::now();
            let result = if stream {
                let app_chunk = app.clone();
                ai::correct_stream(&http, &req, &token, move |delta| {
                    let _ = app_chunk.emit(
                        "provider-chunk",
                        json!({
                            "session_id": session_id,
                            "provider": provider,
                            "delta": delta,
                        }),
                    );
                })
                .await
            } else {
                ai::correct(&http, &req, &token).await
            };

            let elapsed_ms = started.elapsed().as_millis() as u64;
            match result {
                Ok(text) => {
                    let _ = app.emit(
                        "provider-result",
                        json!({
                            "session_id": session_id,
                            "provider": provider,
                            "text": text,
                            "elapsed_ms": elapsed_ms,
                        }),
                    );
                }
                Err(AiError::Cancelled { .. }) => { /* stale session: stay silent */ }
                Err(e) => emit_error(&app, session_id, provider, &e.to_string()),
            }
        });
    }

    session_id
}

fn emit_error(app: &AppHandle, session_id: u64, provider: Provider, message: &str) {
    let _ = app.emit(
        "provider-error",
        json!({
            "session_id": session_id,
            "provider": provider,
            "message": message,
        }),
    );
}

/// Manually start a correction from the UI (e.g. paste-and-correct).
#[tauri::command]
pub fn start_correction(app: AppHandle, text: String, style: String) -> u64 {
    run_correction(&app, text, Style::from_key(&style))
}

/// Cancel the in-flight session (the "Anuluj wszystko" button).
#[tauri::command]
pub fn cancel_session(app: AppHandle) {
    let state = app.state::<AppState>();
    let guard = state.cancel.lock().expect("cancel mutex poisoned");
    if let Some(t) = guard.as_ref() {
        t.cancel();
    }
}
