//! Tauri app entry: registers plugins, shared state, the global hotkey, and the
//! command handlers. Native concerns live in `modules`, pure logic in
//! `poprawiacz-core`.

mod modules;

use modules::{ai, autostart, clipboard, config, hotkey, logging, tray, AppState};
use poprawiacz_core::ai::build_client;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();

    let http = build_client().expect("failed to build HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed
                        && hotkey::is_correction_shortcut(shortcut)
                    {
                        hotkey::on_hotkey(app);
                    }
                })
                .build(),
        )
        .manage(AppState::new(http))
        .setup(|app| {
            // Enforce the frameless window at runtime too — the conf flag can be
            // overridden by restored window state on some setups, leaving the
            // native title bar visible.
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_decorations(false);
                let _ = win.set_shadow(true);
            }
            hotkey::register(app.handle());
            tray::build(app.handle())?;
            // Apply persisted autostart preference at launch (Windows).
            let autostart_on = config::load_settings(app.handle()).autostartup;
            if let Err(e) = autostart::set_enabled(autostart_on) {
                tracing::warn!("autostart sync failed: {e}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window cancels in-flight corrections and hides to the
            // tray instead of quitting (quit via tray "Zakończ"), like Python.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                state.cancel.lock().expect("cancel mutex poisoned").cancel_all();
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            ai::start_correction,
            ai::fe_log,
            ai::reprocess_provider,
            ai::cancel_session,
            ai::cancel_provider,
            clipboard::read_clipboard,
            clipboard::write_clipboard,
            clipboard::paste_text,
            config::get_settings,
            config::save_settings,
            config::migrate_config_ini,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
