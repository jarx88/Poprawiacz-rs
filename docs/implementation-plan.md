# PoprawiaczTekstu (Rust/Tauri) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Greenfield rewrite of the Python (CustomTkinter) "PoprawiaczTekstu" desktop app as a Windows-first Tauri 2 + Rust + React 19/TypeScript/Vite app, preserving the core workflow: `Ctrl+Shift+C` → copy selection → fan out to OpenAI/Anthropic/Gemini/DeepSeek concurrently → show 4 results → paste the chosen one.

**Architecture:** Terax-like boundary. Rust owns OS/native/network/secrets; React owns presentation/state; the boundary is typed Tauri commands + events. **Critical split:** all pure logic (AI request/response mapping, retry/backoff, timeouts, config INI parsing+migration, prompts, log-path resolution) lives in a standalone `poprawiacz-core` library crate with **no `tauri` dependency**, so `cargo test`/`cargo clippy` run on any platform (incl. headless Linux CI). The thin `poprawiacz` app crate depends on `tauri` + `poprawiacz-core` and only compiles on a platform with the desktop GUI stack (Windows, or Linux with `webkit2gtk-4.1`).

**Tech Stack:** Tauri 2, Rust (reqwest+rustls, tokio, futures-util, serde, thiserror, tracing, keyring, configparser), React 19, TypeScript, Vite, Zustand, Vitest, pnpm.

---

## Environment note (this agent's sandbox)

`webkit2gtk-4.1` and `tauri-cli` are absent on the build host, so `pnpm tauri dev`, `pnpm tauri build`, and any `cargo` command that compiles the `tauri`-dependent app crate **cannot run here**. The `poprawiacz-core` crate is designed to be fully testable without that stack: `cargo test -p poprawiacz-core --locked` and `cargo clippy -p poprawiacz-core --all-targets --locked` are the gates verifiable in this environment. Full-app build/dev and the Windows smoke checklist are deferred to a Windows machine (documented in `docs/windows-build.md`).

---

## Feature mapping: Python → Rust/Tauri/React

| Python source | Behavior | New home |
|---|---|---|
| `main_corrector.py` workflow loop | hotkey → copy → 4-way concurrent → results → paste | `src/features/correction/*` (UI/state) + `app crate commands` (orchestration) |
| `api_clients/base_client.py` | timeouts (25s std / 35s DeepSeek / 8s connect), 2 retries, `APIConnectionError`/`APIResponseError`/`APITimeoutError` | `core::ai::{types,retry}`, `core::ai::error` |
| `api_clients/openai_client.py` | Chat Completions, **streaming (SSE)**, session-aware cancel | `core::ai::openai` (stream support) |
| `api_clients/anthropic_client.py` | Messages API, explicit `max_tokens`, `x-api-key` + `anthropic-version` | `core::ai::anthropic` (non-stream MVP) |
| `api_clients/gemini_client.py` | `:generateContent` HTTP, system_instruction, safety BLOCK_NONE | `core::ai::gemini` (non-stream MVP; verified REST endpoint) |
| `api_clients/deepseek_client.py` | OpenAI-compatible, **35s timeout** | `core::ai::deepseek` (non-stream MVP) |
| `utils/config_manager.py` | case-insensitive INI sections/keys, default models | `core::config` |
| `utils/prompts.py` | 7 instruction styles + 3 system prompts | `core::prompts` |
| `utils/hotkey_manager.py` | pynput global hotkey → queue → worker | `app::hotkey` (tauri-plugin-global-shortcut → events) |
| `utils/clipboard_manager.py` | read selection / write+paste | `app::clipboard` (tauri-plugin-clipboard-manager + simulated paste) |
| `utils/logger.py` | `~/PoprawiaczTekstu_logs/popraw_tekst_YYYYMMDD.log`, ERROR level | `core::logging` (path/format) + `app` (tracing subscriber) |
| `config.ini [API_KEYS]` | plaintext keys on disk | **migrated** to OS keychain (`keyring`); `config.ini` keys read once for migration then ignored |

## Provider contracts (verified parity targets)

- **OpenAI** — `POST https://api.openai.com/v1/chat/completions`, `Authorization: Bearer`, body `{model, messages:[{system},{user}], stream}`. Result `choices[0].message.content` (non-stream) / `choices[].delta.content` (SSE `data:` lines, terminated by `data: [DONE]`). Timeout 25s. **Streaming kept (MVP).**
- **Anthropic** — `POST https://api.anthropic.com/v1/messages`, `x-api-key`, `anthropic-version: 2023-06-01`, body `{model, max_tokens, system, messages:[{user}]}`. Result `content[0].text`. Timeout 25s. Non-stream MVP.
- **Gemini** — `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`, header `x-goog-api-key`, body `{system_instruction:{parts:[{text}]}, contents:[{role:"user",parts:[{text}]}], generationConfig:{maxOutputTokens,temperature}, safetySettings:[BLOCK_NONE]}`. Result `candidates[0].content.parts[0].text`. Timeout 25s. Non-stream MVP (REST endpoint verified, distinct from Python SDK).
- **DeepSeek** — `POST https://api.deepseek.com/chat/completions`, OpenAI-compatible. **Timeout 35s (never below 30s).** Non-stream MVP.

Common message shape: system = `core::prompts::system_prompt(style)`; user = `"{instruction}\n\n---\n{text}\n---"`.

Default models (Python defaults): OpenAI `gpt-5-mini`, Anthropic `claude-3-7-sonnet-latest`, Gemini `gemini-2.5-flash`, DeepSeek `deepseek-chat`. Existing `config.ini` may override (e.g. openai `o4-mini`).

## Cancellation / session semantics

- Monotonic `session_id` (u64) incremented on each new correction trigger.
- Each provider task captures its `session_id`. Frontend store **drops** any event (chunk/result/error) whose `session_id != current`. Backend cancels in-flight tasks of the previous session via `tokio_util::sync::CancellationToken` per session, and stale tasks short-circuit on the token.
- Result guard: a provider slot accepts at most one terminal (result|error) per `(session_id, provider)`.

## MVP scope vs later

**MVP:** 4-provider concurrent correction; `Ctrl+Shift+C` hotkey → copy → process; OpenAI streaming + 3 non-stream; per-provider error isolation; session/cancellation; config.ini read + migrate keys to keychain; settings dialog (keys/models/style); choose+paste; logs dir; timeouts+retries; 7 prompt styles; Rust core tests + Vitest; Windows build/smoke docs.

**Later:** diff highlighting, system tray, autostartup (Windows registry), streaming for non-OpenAI (after verification), reasoning-effort/verbosity selectors, original-text popup, custom prompt templates, themes, auto-update/signing/installer.

---

## Task breakdown

### Phase A — Scaffold & quality gates
- [ ] A1. `package.json` (pnpm) with scripts: `dev`, `build`, `tauri`, `test` (vitest), `typecheck` (`tsc --noEmit`).
- [ ] A2. Vite + React 19 + TS config (`vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx`, `src/App.tsx`).
- [ ] A3. Cargo workspace under `src-tauri/`: members `.` (app `poprawiacz`) + `crates/core` (`poprawiacz-core`). Release profile: `lto="fat"`, `opt-level="s"`, `codegen-units=1`, `panic="abort"`, `strip=true`.
- [ ] A4. `tauri.conf.json` (Windows metadata, identifier, window, plugins) + placeholder icons.

### Phase B — `poprawiacz-core` (TDD, runs everywhere)
- [ ] B1. `prompts`: `system_prompt(style)`, `instruction_prompt(style)`, `Style` enum (7 variants) + tests asserting exact strings/fallback.
- [ ] B2. `config`: parse INI case-insensitively, `ApiKeys`/`Models`/`Settings` structs, default models, migration helper. Tests with sample INI incl. mixed-case sections.
- [ ] B3. `ai::types`: `Provider` enum, `CorrectionRequest`, `ProviderResult`, timeouts (`timeout_for(provider)` → 25s/35s), `MAX_RETRIES=2`, `CONNECT_TIMEOUT=8s`.
- [ ] B4. `ai::error`: `AiError` (`Connection`/`Response`/`Timeout`/`Cancelled`) via thiserror + classification tests.
- [ ] B5. `ai::retry`: `with_retries` (2 retries, fixed backoff, no retry on Cancelled/4xx) — tested with a fake fallible fn.
- [ ] B6. Per-provider request builders + response parsers (`openai`/`anthropic`/`gemini`/`deepseek`): pure fns `build_request(...) -> http parts` and `parse_response(json) -> String`. Tests on canned JSON fixtures + SSE chunk parsing for OpenAI.
- [ ] B7. `logging`: `logs_dir()` (`~/PoprawiaczTekstu_logs/`), `log_file_name(date)` format tests.

### Phase C — `poprawiacz` app crate (compiles on Windows / webkit Linux)
- [ ] C1. `lib.rs`: register plugins (global-shortcut, clipboard-manager, store, log, notification, window-state, opener), shared `AppState` (reqwest client, session counter, cancel tokens, keychain handle), `generate_handler![...]`.
- [ ] C2. `modules/config`: tauri commands `get_settings`, `set_settings`, `migrate_config_ini`; keys via `keyring`.
- [ ] C3. `modules/ai`: command `start_correction(text, style)` → spawns 4 tasks, emits `provider-chunk`/`provider-result`/`provider-error` events with `session_id`; `cancel_session`.
- [ ] C4. `modules/clipboard`: `read_selection` / `paste_text` (write clipboard + simulate Ctrl+V).
- [ ] C5. `modules/hotkey`: register `Ctrl+Shift+C`, emit `hotkey-triggered`; debounce/cancel previous session.
- [ ] C6. `modules/logging`: init tracing subscriber to core's logs dir; never log secrets/full text.

### Phase D — React UI
- [ ] D1. `lib/tauri.ts` typed wrappers (commands + event listeners).
- [ ] D2. `features/correction` Zustand store with session isolation (drop stale events) + `ProviderPanel` x4 + streaming indicator + per-provider error.
- [ ] D3. `features/settings` dialog (keys/models/style) with validation; lazy-loaded.
- [ ] D4. App shell renders immediately; panels/settings lazy.

### Phase E — Tests, docs, gates
- [ ] E1. Vitest: store session-isolation + settings validation.
- [ ] E2. `docs/windows-build.md` build instructions + manual smoke checklist + perf/bundle notes vs PyInstaller.
- [ ] E3. Final report: files, gaps, commands run, manual tests still required.

## Risks
- No reference comparison run on Windows in this session (platform-sensitive hotkey/clipboard/streaming) — flagged.
- Gemini/OpenAI model IDs in `config.ini` (`o4-mini`, `gpt-5-mini`) must be validated against live API on Windows.
- Full `cargo test`/`clippy` of the app crate and `tauri dev/build` require the desktop GUI stack (unavailable in this sandbox).
