//! Tauri app entry: registers plugins, shared state, the global hotkey, and the
//! command handlers. Native concerns live in `modules`, pure logic in
//! `poprawiacz-core`.

mod modules;

use modules::{ai, clipboard, config, hotkey, logging, AppState};
use poprawiacz_core::ai::build_client;
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
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed
                        && shortcut == &hotkey::correction_shortcut()
                    {
                        hotkey::on_hotkey(app);
                    }
                })
                .build(),
        )
        .manage(AppState::new(http))
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Err(e) = app.global_shortcut().register(hotkey::correction_shortcut()) {
                tracing::error!("failed to register global shortcut: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ai::start_correction,
            ai::cancel_session,
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
