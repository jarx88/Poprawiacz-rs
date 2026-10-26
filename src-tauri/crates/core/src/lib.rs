//! Platform-agnostic core for PoprawiaczTekstu.
//!
//! Contains all logic that does NOT need the desktop GUI stack: AI provider
//! request building / response parsing / retry / timeouts, INI config parsing
//! and migration, prompt templates, log-path resolution, and the update-manifest
//! checks used by the self-updater. This crate has no
//! `tauri` dependency, so `cargo test`/`cargo clippy` run on any platform,
//! including headless CI.

pub mod ai;
pub mod config;
pub mod logging;
pub mod prompts;
pub mod update;

pub use prompts::Style;
