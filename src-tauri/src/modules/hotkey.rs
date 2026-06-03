//! Global hotkey handling. `Ctrl+Shift+C` (with fallbacks) copies the current
//! selection and starts a new correction session, mirroring the Python pynput
//! flow but event-driven (never blocks the UI thread).

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use super::{ai, clipboard, config};
use poprawiacz_core::Style;

/// The primary global shortcut: Ctrl+Shift+C.
pub fn correction_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyC)
}

/// Primary + fallback shortcuts, tried in order (parity with hotkey_manager.py:
/// Ctrl+Shift+C, Ctrl+Shift+Alt+C, Ctrl+Shift+V, Shift+Alt+C).
pub fn shortcuts() -> Vec<Shortcut> {
    vec![
        correction_shortcut(),
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
            Code::KeyC,
        ),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
        Shortcut::new(Some(Modifiers::SHIFT | Modifiers::ALT), Code::KeyC),
    ]
}

/// Whether a pressed shortcut is one of ours.
pub fn is_correction_shortcut(shortcut: &Shortcut) -> bool {
    shortcuts().iter().any(|s| s == shortcut)
}

/// Register the primary shortcut, falling back to alternatives if it's taken.
pub fn register(app: &AppHandle) {
    let gs = app.global_shortcut();
    for (i, sc) in shortcuts().into_iter().enumerate() {
        match gs.register(sc) {
            Ok(()) => {
                if i == 0 {
                    return; // primary registered; fallbacks not needed
                }
                tracing::warn!("registered fallback hotkey #{i}");
                return;
            }
            Err(e) => tracing::warn!("hotkey #{i} unavailable: {e}"),
        }
    }
    tracing::error!("no global shortcut could be registered");
}

/// Handle a hotkey press: copy selection, read clipboard, start correction,
/// surface the window. Spawned so the shortcut handler returns immediately.
pub fn on_hotkey(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let before = app.clipboard().read_text().unwrap_or_default();

        // Wait for the user to physically release Ctrl/Shift before we
        // synthesize Ctrl+C — otherwise the still-held Shift turns our 'c' into
        // Ctrl+Shift+C again and nothing is copied (this caused "schowek pusty").
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        if let Err(e) = tauri::async_runtime::spawn_blocking(clipboard::simulate_copy)
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r)
        {
            tracing::warn!("simulate_copy failed: {e}");
        }

        // Poll for a fresh copy (changed clipboard) over a few adaptive tries.
        let mut text = String::new();
        for attempt in 0..4u32 {
            tokio::time::sleep(std::time::Duration::from_millis(60 + 50 * attempt as u64)).await;
            let now = app.clipboard().read_text().unwrap_or_default();
            if !now.trim().is_empty() && now != before {
                text = now;
                break;
            }
        }
        // Fallback: use whatever is in the clipboard now (selection may equal
        // the previous content, or the user pre-copied) rather than failing.
        if text.trim().is_empty() {
            let now = app.clipboard().read_text().unwrap_or_default();
            if !now.trim().is_empty() {
                text = now;
            }
        }

        if text.trim().is_empty() {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
            let _ = app.emit("hotkey-empty", ());
            return;
        }

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

    #[test]
    fn recognizes_all_fallbacks() {
        for s in shortcuts() {
            assert!(is_correction_shortcut(&s));
        }
    }
}
