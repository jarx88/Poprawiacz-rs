//! Correction orchestration: fan out to all providers concurrently, emit
//! per-provider streaming/result/error events tagged with the session id.
//! Each provider has its own cancellation token (cancel one or cancel all).

use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};

use futures_util::FutureExt;
use poprawiacz_core::ai::{self, AiError, CorrectionRequest, Provider};
use poprawiacz_core::Style;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use super::config;
use super::AppState;

/// Spawn one provider's correction task within `session_id`, emitting
/// chunk/result/error/cancelled events. Shared by full runs and single-panel
/// reprocessing.
#[allow(clippy::too_many_arguments)]
fn spawn_provider(
    app: &AppHandle,
    session_id: u64,
    provider: Provider,
    text: String,
    style: Style,
    token: CancellationToken,
) {
    let settings = config::load_settings(app);
    let model = settings.models.resolved(provider);
    let reasoning_effort = settings.ai_settings.reasoning_effort.clone();
    let verbosity = settings.ai_settings.verbosity.clone();
    let key = config::get_api_key(provider);
    let http = app.state::<AppState>().http.clone();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        let app_guard = app.clone();
        // Hard ceiling so the panel ALWAYS resolves even if a lower layer hangs.
        let hard = provider.timeout() + Duration::from_secs(15);

        let work = AssertUnwindSafe(async move {
            tracing::info!(provider = provider.key(), session_id, "task: start");
            let Some(api_key) = key else {
                tracing::warn!(provider = provider.key(), "task: no API key");
                emit_error(&app, session_id, provider, "Brak klucza API");
                return;
            };

            let stream = provider.supports_streaming();
            tracing::info!(provider = provider.key(), model = %model, stream, "task: sending request");
            let req = CorrectionRequest {
                provider,
                model,
                api_key,
                style,
                text,
                stream,
                reasoning_effort,
                verbosity,
            };

            let started = Instant::now();
            let result = if stream {
                let app_chunk = app.clone();
                ai::correct_stream(&http, &req, &token, move |delta| {
                    let _ = app_chunk.emit(
                        "provider-chunk",
                        json!({ "session_id": session_id, "provider": provider, "delta": delta }),
                    );
                })
                .await
            } else {
                ai::correct(&http, &req, &token).await
            };

            let elapsed_ms = started.elapsed().as_millis() as u64;
            match result {
                Ok(text) => {
                    tracing::info!(provider = provider.key(), elapsed_ms, len = text.len(), "task: result");
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
                Err(AiError::Cancelled { .. }) => {
                    tracing::info!(provider = provider.key(), "task: cancelled");
                    let _ = app.emit(
                        "provider-cancelled",
                        json!({ "session_id": session_id, "provider": provider }),
                    );
                }
                Err(e) => {
                    tracing::error!(provider = provider.key(), elapsed_ms, "task: error: {e}");
                    emit_error(&app, session_id, provider, &e.to_string());
                }
            }
        });

        // Run with panic-capture + a hard timeout so the UI never spins forever.
        match tokio::time::timeout(hard, work.catch_unwind()).await {
            Ok(Ok(())) => {}
            Ok(Err(_panic)) => {
                tracing::error!(provider = provider.key(), "task: PANICKED");
                emit_error(&app_guard, session_id, provider, "Wewnętrzny błąd (panic) — sprawdź log");
            }
            Err(_elapsed) => {
                tracing::error!(provider = provider.key(), secs = hard.as_secs(), "task: HARD TIMEOUT");
                emit_error(
                    &app_guard,
                    session_id,
                    provider,
                    &format!("Przekroczono czas ({}s)", hard.as_secs()),
                );
            }
        }
    });
}

/// Start a new correction session for `text`. Cancels the previous session,
/// then spawns one task per provider. Returns the new session id.
pub fn run_correction(app: &AppHandle, text: String, style: Style) -> u64 {
    let state = app.state::<AppState>();
    let session_id = state
        .session
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;

    // Fresh per-provider tokens; cancel the previous session's in-flight tasks.
    let mut tokens = std::collections::HashMap::new();
    {
        let mut guard = state.cancel.lock().expect("cancel mutex poisoned");
        guard.cancel_all();
        for provider in Provider::ALL {
            tokens.insert(provider, CancellationToken::new());
        }
        guard.tokens = tokens.clone();
    }
    // Remember the input so a single panel can be reprocessed later.
    *state.last_input.lock().expect("last_input poisoned") =
        Some((session_id, text.clone(), style));

    let _ = app.emit(
        "session-started",
        json!({ "session_id": session_id, "text": text }),
    );

    for provider in Provider::ALL {
        spawn_provider(app, session_id, provider, text.clone(), style, tokens[&provider].clone());
    }

    session_id
}

fn emit_error(app: &AppHandle, session_id: u64, provider: Provider, message: &str) {
    let _ = app.emit(
        "provider-error",
        json!({ "session_id": session_id, "provider": provider, "message": message }),
    );
}

/// Manually start a correction from the UI (e.g. paste-and-correct).
#[tauri::command]
pub fn start_correction(app: AppHandle, text: String, style: String) -> u64 {
    run_correction(&app, text, Style::from_key(&style))
}

/// Re-run a single provider in the current session with a (possibly different)
/// style — the per-panel ⚙️ action menu (professional / EN / PL).
#[tauri::command]
pub fn reprocess_provider(app: AppHandle, provider: String, style: String) -> Result<(), String> {
    let Some(p) = Provider::ALL.into_iter().find(|p| p.key() == provider) else {
        return Err(format!("unknown provider: {provider}"));
    };
    let state = app.state::<AppState>();
    let (session_id, text) = {
        let guard = state.last_input.lock().expect("last_input poisoned");
        let Some((sid, text, _)) = guard.as_ref() else {
            return Err("no active session".into());
        };
        (*sid, text.clone())
    };

    // Fresh token for this provider; cancel its previous in-flight task.
    let token = CancellationToken::new();
    {
        let mut guard = state.cancel.lock().expect("cancel mutex poisoned");
        guard.cancel_one(p);
        guard.tokens.insert(p, token.clone());
    }
    let _ = app.emit(
        "provider-restarted",
        json!({ "session_id": session_id, "provider": p }),
    );
    spawn_provider(&app, session_id, p, text, Style::from_key(&style), token);
    Ok(())
}

/// Cancel all in-flight providers (the "Anuluj wszystko" button).
#[tauri::command]
pub fn cancel_session(app: AppHandle) {
    let state = app.state::<AppState>();
    let mut guard = state.cancel.lock().expect("cancel mutex poisoned");
    guard.cancel_all();
}

/// Cancel a single provider's in-flight request (the per-panel ✖ button).
#[tauri::command]
pub fn cancel_provider(app: AppHandle, provider: String) {
    let Some(p) = Provider::ALL.into_iter().find(|p| p.key() == provider) else {
        return;
    };
    let state = app.state::<AppState>();
    let guard = state.cancel.lock().expect("cancel mutex poisoned");
    guard.cancel_one(p);
}
