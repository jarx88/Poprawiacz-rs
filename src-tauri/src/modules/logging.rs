//! Tracing init. Logs go to `~/PoprawiaczTekstu_logs/` (path from core) oraz na
//! stdout (parytet z Pythonem: konsola + plik). Secrets i full private text
//! never logged.

use std::fs::OpenOptions;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// Initialise logging do pliku ORAZ na stdout. Best-effort: failures are
/// swallowed so logging never crashes the app.
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

    // Plik: bez ANSI (kolory smiecilyby w logu).
    let file_layer = fmt::layer()
        .with_writer(file)
        .with_ansi(false)
        .with_filter(LevelFilter::INFO);

    // Stdout: z ANSI dla czytelnosci w konsoli.
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_filter(LevelFilter::INFO);

    // Layered Registry: oba layery rownolegle. try_init nie panikuje, gdy
    // globalny subscriber jest juz ustawiony.
    let _ = tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init();
}
