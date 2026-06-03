# Windows build & smoke checklist

PoprawiaczTekstu is **Windows-first**. Development and CI can also run on Linux
(the `poprawiacz-core` crate and the whole frontend build/test there), but the
hotkey / clipboard / paste integration must be verified on Windows.

## Prerequisites (Windows)

1. **Rust** (stable, MSVC toolchain): https://rustup.rs → `x86_64-pc-windows-msvc`.
2. **Microsoft C++ Build Tools** (Desktop development with C++).
3. **WebView2 Runtime** — preinstalled on Windows 11; on older systems install
   the Evergreen runtime from Microsoft.
4. **Node 20+** and **pnpm** (`npm i -g pnpm` or Corepack).

## Prerequisites (Linux dev — already provisioned on this machine)

Installed via apt on Debian 13:

```bash
sudo apt-get install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev libayatana-appindicator3-dev \
  curl wget file
```

`libxdo-dev` is required by `enigo` for Ctrl+C / Ctrl+V simulation on X11.

## Install & run

```bash
pnpm install            # JS deps (allows the esbuild build script)
pnpm tauri dev          # dev app with hot reload
pnpm tauri build        # compile the portable .exe (no installer)
```

### Portable single-file build (default)

`bundle.active` is set to `false` in `tauri.conf.json`, so `pnpm tauri build`
compiles a **single self-contained executable** and skips installer generation.
The frontend (HTML/JS/CSS) is embedded into the binary.

- Windows: `src-tauri/target/release/poprawiacz.exe`
- Linux:   `src-tauri/target/release/poprawiacz`

Copy that one file anywhere and run it — no install step. Size: ~7–10 MB
(vs ~49 MB for the old PyInstaller single-exe).

**Only runtime dependency: WebView2**, which is preinstalled on Windows 11 and
nearly all Windows 10. On a rare machine without it, install Microsoft's small
Evergreen WebView2 runtime once.

### If you later want an installer (NSIS/MSI) instead

Set `"bundle": { "active": true }` in `tauri.conf.json` and rebuild;
`pnpm tauri build` will then also emit `target/release/bundle/nsis/*.exe`.

## Quality gates (run before every PR)

```bash
pnpm exec tsc --noEmit                                   # frontend types
pnpm test                                                # Vitest (store + settings)
cd src-tauri
cargo test --workspace --locked                          # Rust unit tests (44)
cargo clippy --workspace --all-targets --locked -- -D warnings
```

> Note: `cargo test --locked` alone tests only the `poprawiacz` app crate.
> Use `--workspace` to also run the 43 `poprawiacz-core` tests (the pure logic:
> prompts, config parsing, provider request/response mapping, retry, SSE).

## First-run configuration

1. Open **⚙️ Ustawienia**.
2. Either paste API keys per provider, or click **📥 Migruj config.ini** and
   point it at the old Python `config.ini`. Keys are stored in the **Windows
   Credential Manager** (via `keyring`), never on disk or in localStorage.
3. Optionally adjust models and the default correction style.

## Manual smoke checklist (Windows)

- [ ] App launches; main window renders immediately (shell before panels).
- [ ] `Ctrl+Shift+C` while text is selected in another app → window appears and a
      new session starts (session counter increments).
- [ ] All four providers run concurrently; OpenAI streams token-by-token while
      the others show a spinner.
- [ ] Killing one provider (e.g. wrong key) shows an error in **only that panel**;
      the other three still complete.
- [ ] Triggering `Ctrl+Shift+C` again mid-run cancels the old session; stale
      results never overwrite the new panels.
- [ ] **📋 Użyj <provider>** writes the result to the clipboard, hides the window
      and pastes into the previously focused app.
- [ ] **❌ Anuluj wszystko** cancels in-flight requests.
- [ ] Logs appear under `%USERPROFILE%\PoprawiaczTekstu_logs\popraw_tekst_YYYYMMDD.log`
      and contain **no API keys and no full corrected text**.

## Performance / bundle notes vs. Python (PyInstaller)

| Aspect | Old (PyInstaller) | New (Tauri 2) — expected |
|---|---|---|
| Installer / binary size | ~49 MB single-exe | ~3–10 MB (system WebView2, `lto="fat"`, `opt-level="s"`, `strip`, `panic="abort"`) |
| Cold start | seconds (Python + CustomTkinter + interpreter unpack) | sub-second native window |
| Memory | high (CPython + Tk) | low (native Rust + system WebView) |
| Concurrency | Python threads per provider | native async (`tokio`) tasks, shared pooled HTTP client |

Numbers for the new app are projections from the release profile and the Terax
reference; measure on the target Windows machine after `pnpm tauri build`.
