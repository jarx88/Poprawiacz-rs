//! Embeds the build identity used by the self-updater. `scripts/publish-dev.sh`
//! sets both variables so the EXE and its `latest.json` agree; plain builds
//! fall back to the git commit and the current time.

use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-env-changed=POPRAWIACZ_BUILD");
    println!("cargo:rerun-if-env-changed=POPRAWIACZ_BUILT_AT");

    let built_at = std::env::var("POPRAWIACZ_BUILT_AT")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let build = std::env::var("POPRAWIACZ_BUILD")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("{}-{built_at}", git_short_sha().unwrap_or_else(|| "local".into())));
    println!("cargo:rustc-env=POPRAWIACZ_BUILD={build}");
    println!("cargo:rustc-env=POPRAWIACZ_BUILT_AT={built_at}");

    tauri_build::build()
}

fn git_short_sha() -> Option<String> {
    let out = std::process::Command::new("git").args(["rev-parse", "--short", "HEAD"]).output().ok()?;
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !sha.is_empty()).then_some(sha)
}
