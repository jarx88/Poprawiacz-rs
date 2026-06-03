//! Global hotkey handling. `Ctrl+Shift+C` copies the current selection and
//! starts a new correction session, mirroring the Python pynput flow but
//! event-driven (never blocks the UI thread).

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

use super::{ai, clipboard, config};
use poprawiacz_core::Style;

/// The primary global shortcut: Ctrl+Shift+C.
pub fn correction_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyC)
}

/// Handle a hotkey press: copy selection, read clipboard, start correction,
/// surface the window. Spawned so the shortcut handler returns immediately.
pub fn on_hotkey(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Simulate Ctrl+C off the async thread, then wait for the clipboard.
        if let Err(e) = tauri::async_runtime::spawn_blocking(clipboard::simulate_copy)
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r)
        {
            tracing::warn!("simulate_copy failed: {e}");
        }
        tokio::time::sleep(clipboard::COPY_SETTLE).await;

        let text = match app.clipboard().read_text() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("clipboard read failed: {e}");
                String::new()
            }
        };

        if text.trim().is_empty() {
            let _ = app.emit("hotkey-empty", ());
            return;
        }

        // Show & focus the main window.
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }

        let style = Style::from_key(&config::load_settings(&app).default_style);
        ai::run_correction(&app, text, style);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_is_ctrl_shift_c() {
        let s = correction_shortcut();
        assert_eq!(s.key, Code::KeyC);
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::SHIFT);
    }
}
