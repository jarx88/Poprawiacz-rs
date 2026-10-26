//! Self-update from the dev machine over Tailscale, ported from HyperRust (see
//! `scripts/publish-dev.sh` and `poprawiacz_core::update`). Newer builds are
//! downloaded in the background, verified, and swapped in only while the app
//! sits in the tray: the running EXE is renamed to `*.exe.old` (Windows allows
//! renaming a running image), the new one takes its place, and it is started
//! with `--after-update <pid>` so it waits for this process to exit (the global
//! hotkey can only be registered by one process).
//!
//! The previous build also starts itself from `*.exe.old` as a guard
//! (`--update-guard <pid>`). If the new build exits or hangs before it reports
//! readiness, the guard puts the previous EXE back, remembers the failed
//! build's SHA-256 so it is not installed again, and restarts the old version.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use poprawiacz_core::update::{self, BuildInfo, Manifest};
use tauri::{AppHandle, Manager};

use super::AppState;

pub const BUILD: BuildInfo = BuildInfo {
    version: env!("CARGO_PKG_VERSION"),
    build: env!("POPRAWIACZ_BUILD"),
    built_at: parse_u64(env!("POPRAWIACZ_BUILT_AT")),
};

const fn parse_u64(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut n = 0u64;
    while i < bytes.len() {
        n = n * 10 + (bytes[i] - b'0') as u64;
        i += 1;
    }
    n
}

const AFTER_UPDATE_ARG: &str = "--after-update";
const GUARD_ARG: &str = "--update-guard";
/// Install only this long after the last correction started.
const QUIET: Duration = Duration::from_secs(60);
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(8);
const CHECK_INTERVAL: Duration = Duration::from_secs(120);
/// How long the previous build waits to exit before the new one stops it.
const OLD_EXIT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a freshly installed build has to finish starting up.
const STARTUP_GRACE: Duration = Duration::from_secs(60);

/// Path where a verified update waits next to the running EXE.
fn staged_path(exe: &Path) -> PathBuf {
    exe.with_extension("update.exe")
}

fn old_path(exe: &Path) -> PathBuf {
    exe.with_extension("exe.old")
}

/// Written by a freshly installed build once it is up; the guard waits for it.
fn ready_marker(exe: &Path) -> PathBuf {
    exe.with_extension("update-ok")
}

/// SHA-256 of a build that failed to start; it is never installed again.
fn skip_file(exe: &Path) -> PathBuf {
    exe.with_extension("update-skip")
}

fn failed_path(exe: &Path) -> PathBuf {
    exe.with_extension("failed.exe")
}

pub async fn run(app: AppHandle) {
    if !cfg!(windows) {
        return; // the EXE swap below is Windows-only
    }
    let client = match update::client() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("aktualizacje: klient HTTP: {e}");
            return;
        }
    };
    tokio::time::sleep(FIRST_CHECK_DELAY).await;
    loop {
        if let Some((staged, manifest)) = check_and_download(&client).await {
            wait_until_idle(&app).await;
            match apply(&staged) {
                Ok(()) => {
                    tracing::info!("aktualizacja {} zainstalowana, restartuję", manifest.build);
                    app.exit(0);
                    return;
                }
                Err(e) => tracing::warn!("instalacja aktualizacji: {e}"),
            }
        }
        tokio::time::sleep(CHECK_INTERVAL).await;
    }
}

/// Returns the path of a verified, staged EXE when a newer build exists.
async fn check_and_download(client: &reqwest::Client) -> Option<(PathBuf, Manifest)> {
    let url = update::validate_url(update::DEFAULT_UPDATE_URL).ok()?;
    let manifest = match update::fetch_manifest(client, &url).await {
        Ok(m) => m,
        Err(e) => {
            // Expected off the tailnet; not worth a warning every interval.
            tracing::debug!("sprawdzanie aktualizacji: {e}");
            return None;
        }
    };
    if !manifest.is_newer_than(&BUILD) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let skipped = std::fs::read_to_string(skip_file(&exe)).unwrap_or_default();
    if skipped.trim() == manifest.sha256 {
        tracing::debug!("build {} został wcześniej wycofany, pomijam", manifest.build);
        return None;
    }
    let staged = staged_path(&exe);
    let already = std::fs::read(&staged).ok().is_some_and(|b| update::verify(&b, &manifest).is_ok());
    if !already {
        tracing::info!("pobieranie aktualizacji {}", manifest.build);
        let bytes = match update::download_url(&url, &manifest) {
            Ok(dl) => update::download_verified(client, &dl, &manifest).await,
            Err(e) => Err(e),
        };
        let bytes = match bytes {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("pobieranie aktualizacji: {e}");
                return None;
            }
        };
        if let Err(e) = update::write_atomically(&staged, &bytes) {
            tracing::warn!("zapis {}: {e}", staged.display());
            return None;
        }
    }
    tracing::info!("aktualizacja {} gotowa ({})", manifest.build, manifest.notes);
    Some((staged, manifest))
}

/// Wait until the window is hidden in the tray and no correction started for
/// [`QUIET`], so a restart never throws away results the user is looking at.
// shortcut: a hotkey press in the instant between this check and exit is lost
// (the user presses again); add an install lock in `run_correction` if it bites.
async fn wait_until_idle(app: &AppHandle) {
    loop {
        let hidden = app
            .get_webview_window("main")
            .map_or(true, |w| !w.is_visible().unwrap_or(true));
        let last = app.state::<AppState>().last_activity_s.load(Ordering::SeqCst);
        if hidden && now_s().saturating_sub(last) >= QUIET.as_secs() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

pub fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Swap the EXE, start the new build and its guard. On error everything is
/// rolled back.
fn apply(staged: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::remove_file(ready_marker(&exe));
    std::fs::rename(&exe, &old).map_err(|e| format!("zmiana nazwy bieżącego programu: {e}"))?;
    if let Err(e) = std::fs::rename(staged, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("podmiana programu: {e}"));
    }
    let mut child = match std::process::Command::new(&exe)
        .arg(AFTER_UPDATE_ARG)
        .arg(std::process::id().to_string())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::rename(&exe, staged);
            let _ = std::fs::rename(&old, &exe);
            return Err(format!("uruchomienie nowej wersji: {e}"));
        }
    };
    // Our own image now lives at `old`; run it as the guard of the new build.
    // Without a guard a broken build could not be rolled back, so the
    // install is abandoned (the new process is still waiting for us to exit).
    if let Err(e) = std::process::Command::new(&old)
        .arg(GUARD_ARG)
        .arg(child.id().to_string())
        .arg(std::process::id().to_string())
        .spawn()
    {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::rename(&exe, staged);
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("uruchomienie strażnika aktualizacji: {e}"));
    }
    Ok(())
}

/// Entry point of `--update-guard <new-pid> <previous-pid>`, run from the
/// previous build's `*.exe.old`. Returns `true` when this process was the
/// guard and must exit.
pub fn run_guard(args: &[String]) -> bool {
    let Some(pos) = args.iter().position(|a| a == GUARD_ARG) else { return false };
    let pid = |i: usize| args.get(pos + i).and_then(|p| p.parse::<u32>().ok());
    let (Some(new_pid), Some(previous_pid)) = (pid(1), pid(2)) else { return true };
    let Ok(old) = std::env::current_exe() else { return true };
    let exe = old.with_extension(""); // PoprawiaczTekstu.exe.old -> PoprawiaczTekstu.exe
    let marker = ready_marker(&exe);
    match platform::watch_startup(new_pid, || marker.exists(), STARTUP_GRACE) {
        platform::Startup::Ready => {
            let _ = std::fs::remove_file(&marker);
        }
        outcome => {
            tracing::error!("nowa wersja nie wystartowała ({outcome:?}), przywracam poprzednią");
            // The restored build must not meet the previous process still
            // holding the global hotkey.
            platform::wait_for_exit(previous_pid, OLD_EXIT_TIMEOUT);
            if let Err(e) = roll_back(&exe, &old) {
                tracing::error!("wycofanie aktualizacji: {e}");
            }
        }
    }
    true
}

fn roll_back(exe: &Path, old: &Path) -> Result<(), String> {
    if let Ok(bytes) = std::fs::read(exe) {
        let _ = std::fs::write(skip_file(exe), update::sha256_hex(&bytes));
    }
    let failed = failed_path(exe);
    let _ = std::fs::remove_file(&failed);
    std::fs::rename(exe, &failed).map_err(|e| format!("odsunięcie nieudanej wersji: {e}"))?;
    std::fs::rename(old, exe).map_err(|e| format!("przywrócenie poprzedniej wersji: {e}"))?;
    std::process::Command::new(exe)
        .spawn()
        .map_err(|e| format!("uruchomienie poprzedniej wersji: {e}"))?;
    Ok(())
}

/// Called first thing in `run` of a freshly installed build: wait until the
/// previous process (which still holds the global hotkey) has exited.
pub fn finish_pending_update(args: &[String]) -> bool {
    let Some(pos) = args.iter().position(|a| a == AFTER_UPDATE_ARG) else { return false };
    if let Some(pid) = args.get(pos + 1).and_then(|p| p.parse::<u32>().ok()) {
        platform::wait_for_exit(pid, OLD_EXIT_TIMEOUT);
    }
    true
}

/// The app is up: tell the guard, forget a previously failed build and remove
/// the leftovers of the swap once the guard has released them.
pub fn startup_complete(after_update: bool) {
    let Ok(exe) = std::env::current_exe() else { return };
    if after_update {
        if let Err(e) = std::fs::write(ready_marker(&exe), BUILD.build) {
            tracing::warn!("znacznik gotowości aktualizacji: {e}");
        }
        let _ = std::fs::remove_file(skip_file(&exe));
    }
    let leftovers: Vec<PathBuf> =
        [old_path(&exe), failed_path(&exe)].into_iter().filter(|p| p.exists()).collect();
    if leftovers.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if leftovers.iter().all(|p| std::fs::remove_file(p).is_ok() || !p.exists()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

#[cfg(windows)]
mod platform {
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    #[derive(Debug)]
    pub enum Startup {
        Ready,
        Exited,
        TimedOut,
    }

    /// Our own builds only: a recycled PID must never be terminated. The image
    /// must be exactly our EXE or its `*.exe.old` (the previous build).
    fn is_ours(handle: HANDLE) -> bool {
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        // SAFETY: buffer and length describe valid writable memory.
        if unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) } == 0 {
            return false;
        }
        let image = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        let Ok(current) = std::env::current_exe() else { return false };
        let current = current.to_string_lossy().to_lowercase();
        let exe = current.strip_suffix(".old").unwrap_or(&current);
        image == exe || image == format!("{exe}.old")
    }

    fn open(pid: u32) -> Option<HANDLE> {
        let access = PROCESS_SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION;
        // SAFETY: plain Win32 call; the handle is checked for null.
        let handle = unsafe { OpenProcess(access, 0, pid) };
        (!handle.is_null()).then_some(handle)
    }

    pub fn wait_for_exit(pid: u32, timeout: Duration) {
        let Some(handle) = open(pid) else { return }; // already gone
        // SAFETY: valid process handle, closed below.
        unsafe {
            if WaitForSingleObject(handle, timeout.as_millis() as u32) == WAIT_TIMEOUT && is_ours(handle) {
                // The previous build ignored its exit; it would keep the
                // global hotkey and the new build could not register it.
                TerminateProcess(handle, 1);
                WaitForSingleObject(handle, 5_000);
            }
            CloseHandle(handle);
        }
    }

    pub fn watch_startup(pid: u32, ready: impl Fn() -> bool, grace: Duration) -> Startup {
        let Some(handle) = open(pid) else {
            return if ready() { Startup::Ready } else { Startup::Exited };
        };
        let deadline = Instant::now() + grace;
        let outcome = loop {
            if ready() {
                break Startup::Ready;
            }
            // SAFETY: valid process handle.
            if unsafe { WaitForSingleObject(handle, 200) } == WAIT_OBJECT_0 {
                break if ready() { Startup::Ready } else { Startup::Exited };
            }
            if Instant::now() >= deadline {
                // SAFETY: valid process handle opened with PROCESS_TERMINATE.
                unsafe {
                    if is_ours(handle) {
                        TerminateProcess(handle, 1);
                        WaitForSingleObject(handle, 5_000);
                    }
                }
                break Startup::TimedOut;
            }
        };
        // SAFETY: valid handle, closed once.
        unsafe { CloseHandle(handle) };
        outcome
    }
}

#[cfg(not(windows))]
mod platform {
    use std::time::Duration;

    #[derive(Debug)]
    pub enum Startup {
        Ready,
        #[allow(dead_code)]
        Exited,
    }

    pub fn wait_for_exit(_pid: u32, _timeout: Duration) {}

    pub fn watch_startup(_pid: u32, _ready: impl Fn() -> bool, _grace: Duration) -> Startup {
        Startup::Ready
    }
}
