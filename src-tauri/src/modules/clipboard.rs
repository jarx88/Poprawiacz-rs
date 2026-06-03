//! Clipboard read/write + native copy/paste key simulation.
//!
//! Reading selected text from the foreground app requires simulating Ctrl+C and
//! then reading the clipboard; pasting requires writing the clipboard and
//! simulating Ctrl+V. Key simulation uses `enigo` (Windows/macOS/X11).

use std::time::Duration;

use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Delay after hiding our window before simulating Ctrl+V into the target app.
pub const PASTE_SETTLE: Duration = Duration::from_millis(200);

fn with_modifier(key_char: char) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
    let res = enigo.key(Key::Unicode(key_char), Click);
    // Always release Control even if the click failed.
    let _ = enigo.key(Key::Control, Release);
    res.map_err(|e| e.to_string())
}

pub fn simulate_copy() -> Result<(), String> {
    with_modifier('c')
}

pub fn simulate_paste() -> Result<(), String> {
    with_modifier('v')
}

#[tauri::command]
pub fn read_clipboard(app: AppHandle) -> Result<String, String> {
    app.clipboard().read_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

/// Write `text` to the clipboard, hide our window, then paste into whatever app
/// had focus before. Runs the blocking key simulation off the main thread.
#[tauri::command]
pub async fn paste_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    tokio::time::sleep(PASTE_SETTLE).await;
    tauri::async_runtime::spawn_blocking(simulate_paste)
        .await
        .map_err(|e| e.to_string())?
}
