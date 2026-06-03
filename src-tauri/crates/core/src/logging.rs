//! Log-path resolution, parity with `utils/logger.py`.
//!
//! Logs live in `~/PoprawiaczTekstu_logs/`, one file per day named
//! `popraw_tekst_YYYYMMDD.log`. Secrets and full private text must never be
//! written here (enforced at call sites in the app crate).

use chrono::Local;
use std::path::PathBuf;

/// Directory holding the rolling log files: `~/PoprawiaczTekstu_logs/`.
pub fn logs_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("PoprawiaczTekstu_logs"))
}

/// Daily log file name for a given `YYYYMMDD` date string.
pub fn log_file_name(yyyymmdd: &str) -> String {
    format!("popraw_tekst_{yyyymmdd}.log")
}

/// Today's log file name in local time.
pub fn today_log_file_name() -> String {
    log_file_name(&Local::now().format("%Y%m%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_format() {
        assert_eq!(log_file_name("20260603"), "popraw_tekst_20260603.log");
    }

    #[test]
    fn logs_dir_ends_with_expected_folder() {
        if let Some(d) = logs_dir() {
            assert!(d.ends_with("PoprawiaczTekstu_logs"));
        }
    }

    #[test]
    fn today_file_name_is_well_formed() {
        let name = today_log_file_name();
        assert!(name.starts_with("popraw_tekst_"));
        assert!(name.ends_with(".log"));
        assert_eq!(name.len(), "popraw_tekst_YYYYMMDD.log".len());
    }
}
