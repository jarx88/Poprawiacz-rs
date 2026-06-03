//! System tray: icon + context menu (Pokaż / Ustawienia / Zakończ), left-click
//! shows the window. Mirrors the Python app's tray behavior.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Pokaż okno", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ustawienia", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &settings, &separator, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("default window icon missing")
                .clone(),
        )
        .tooltip("PoprawiaczTekstu — Ctrl+Shift+C")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            // Frontend (App.tsx) nasłuchuje "open-settings" i otwiera dialog ustawień.
            "settings" => {
                show_main(app);
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Show, unminimize and focus the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        // Krótko wymuszamy "always on top", żeby podnieść okno nad inne aplikacje,
        // a zaraz potem zdejmujemy flagę (best-effort, ignorujemy błędy).
        let _ = w.set_always_on_top(true);
        let _ = w.set_always_on_top(false);
    }
}
