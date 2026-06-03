//! Tracing init. Logs go to `~/PoprawiaczTekstu_logs/` (path from core). Secrets
//! and full private text must never be logged.

use std::fs::OpenOptions;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;

/// Initialise file logging. Best-effort: failures are swallowed so logging never
/// crashes the app.
pub fn init() {
    let Some(dir) = poprawiacz_core::logging::logs_dir() else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(poprawiacz_core::logging::today_log_file_name());
    let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = fmt()
        .with_writer(file)
        .with_ansi(false)
        .with_max_level(LevelFilter::INFO)
        .try_init();
}
