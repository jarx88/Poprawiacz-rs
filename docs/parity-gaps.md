# Parity Gaps — Python → Rust/Tauri (Poprawiacz-rs)

> Source: audited parity matrix (88 feature rows across 6 areas: `hotkey-session`,
> `ai-streaming`, `ui-panels-diff`, `settings-config`, `clipboard-tray-window`,
> `prompts-styles`, `logging-errors-misc`).
> Goal per project mandate: **FULL parity — no "later" list.** Everything below must ship.

## 1. Overall parity

| Status | Count | Share |
|---|---:|---:|
| `done` | 39 | ~44% |
| `partial` | 26 | ~30% |
| `missing` | 23 | ~26% |
| **Total** | **88** | **100%** |

Roughly **44% fully done**. The remaining **56%** (49 rows) is split between partials
(behaviorally close but feature-incomplete) and outright gaps. Several rows describe the
**same underlying feature seen from two audit areas** (e.g. `HighlightDiffs`, autostart,
tray menu, progress-bar animation) — those collapse to a single workstream, so the real
unit-of-work count is lower than 49. The hard core (`must`-priority gaps) is small: only
the **Responses API (GPT-5/o1)** is a `must`+`missing`; the rest of the `must` work is
finishing partials (cancel-all UI feedback, settings UI, clipboard retry, minimize-on-close
cancel, status labels, API error logging).

## 2. All missing/partial features (sorted: priority must > should > nice, then effort S < M < L)

| Feature | Status | Python behavior | Target file(s) in new app | Effort | Priority |
|---|---|---|---|---|---|
| Responses API (GPT-5/o1, not Chat Completions) | missing | openai_client.py:129-246 routes gpt-5*/o1* to `client.responses.stream/create`, variant names, reasoning+verbosity | `src-tauri/crates/core/src/ai/openai.rs` (+ routing in `mod.rs`, `src/modules/ai.rs`) | L | must |
| Cancel-all button + per-panel UI reset (❌ Anulowano) | partial | main_corrector.py:1726-1765 sets all flags, resets 4 panels to "❌ Anulowano", progress 0% | `src-tauri/src/modules/ai.rs:125`, `src/features/correction/CorrectionView.tsx`, `store.ts` | S | must |
| Panel header status label format (time / ❌) | partial | api_labels show "API ({time}s)" / "❌ API" in header | `src/features/correction/ProviderPanel.tsx:9-24` | S | must |
| Settings UI completeness (HighlightDiffs etc. exposed) | partial | SettingsWindow exposes all config incl. HighlightDiffs | `src/features/correction/SettingsDialog.tsx`, `config.rs` | M | must |
| Clipboard read via simulated Ctrl+C with retry (3× adaptive) | partial | main_corrector.py:1385-1422 retries 3× with 40-80ms backoff before settle | `src-tauri/src/modules/hotkey.rs:23-30`, `clipboard.rs` | M | must |
| Window minimize-to-tray cancels in-flight corrections | partial | main_corrector.py:2057-2070 cancels all APIs on close before withdraw | `src-tauri/src/lib.rs:43-49`, `src/modules/ai.rs` | M | must |
| API-specific error logging (status/body to file) | partial | utils/logger.py:63-84 log_api_error/connection/timeout to file | `src-tauri/crates/core/src/ai/error.rs`, `src/modules/ai.rs:98`, `modules/logging.rs` | M | must |
| Fallback hotkeys (3 alternatives) | missing | hotkey_manager.py:168-199 tries Ctrl+Shift+Alt+C, Ctrl+Shift+V, Shift+Alt+C | `src-tauri/src/modules/hotkey.rs:13-15` | M | should |
| Per-API cancel (individual provider) | missing | main_corrector.py:1708-1723 cancel_single_api(idx), per-thread flag | `src-tauri/src/modules/ai.rs`, `ProviderPanel.tsx`, `CorrectionView.tsx` | M | should |
| Worker queue for clipboard ops | partial | hotkey_manager.py:16 queue.Queue + worker loop | `src-tauri/src/modules/hotkey.rs:19-30` (async spawn) | M | should |
| Streaming: Anthropic Messages API | missing | anthropic_client.py:61-87 messages.stream(), content_block_delta | `src-tauri/crates/core/src/ai/anthropic.rs`, `types.rs:65`, `mod.rs` | M | should |
| Streaming: Gemini generativeAI | missing | gemini_client.py:164-193 generate_content_stream + _StreamingState | `src-tauri/crates/core/src/ai/gemini.rs`, `types.rs:65` | M | should |
| Streaming: DeepSeek Chat Completions | missing | deepseek_client.py:88-119 SSE `data:` parsing | `src-tauri/crates/core/src/ai/deepseek.rs`, `types.rs:65` | M | should |
| reasoning_effort parameter | missing | openai_client.py:142,220 reasoning={'effort':...} | `config.rs` (AISettings), `openai.rs`, `SettingsDialog.tsx` | S | should |
| verbosity parameter | missing | openai_client.py:143,221 text={'verbosity':...} | `config.rs` (AISettings), `openai.rs`, `SettingsDialog.tsx` | S | should |
| ReasoningEffort config persistence + UI | missing | config.ini [AI_SETTINGS], SettingsWindow dropdown | `src-tauri/crates/core/src/config.rs`, `SettingsDialog.tsx` | M | should |
| Verbosity config persistence + UI | missing | config.ini [AI_SETTINGS], SettingsWindow dropdown | `src-tauri/crates/core/src/config.rs`, `SettingsDialog.tsx` | M | should |
| Retry with exponential backoff (verify semantics) | partial | base_client.py:21-22 DEFAULT_RETRIES=2 (SDK defaults) | `src-tauri/crates/core/src/ai/retry.rs`, `types.rs:15`, `mod.rs:112-114` | M | should |
| Animated GIF / spinner loader | missing | AnimatedGIF class main_corrector.py:298-404, 100ms frames | `src/features/correction/ProviderPanel.tsx` (loading state) | M | should |
| Diff highlighting (red underline of changes) | missing | _highlight_diff() 1299-1329 difflib SequenceMatcher, #d93025 underline | `src/features/correction/ProviderPanel.tsx:41-43`, new diff util | M | should |
| Per-panel action menu (⚙️: professional / EN / PL) | missing | show_action_menu() 1778-1848, reprocess_single_panel() | `src/features/correction/ProviderPanel.tsx`, `src/modules/ai.rs` | L | should |
| Original text window (📄 button) | missing | show_original_text_window() 653-757, read-only + copy | new modal component, `store.ts:22`, `CorrectionView.tsx` | M | should |
| Cancel button per panel (✖) | missing | cancel_btn 884-897 → cancel_single_api(idx) | `src/features/correction/ProviderPanel.tsx` | S | should |
| Autostart registry manipulation | missing | config_manager.py 247-335 winreg Run key | new `src-tauri/src/modules/autostart.rs`, `SettingsDialog.tsx` | M | should |
| AutoStartup setting wired to registry | partial | config.ini [SETTINGS] AutoStartup + tray toggle | `config.rs:104`, `SettingsDialog.tsx`, autostart module | M | should |
| HighlightDiffs setting + render (UI toggle) | partial | config_manager.py:163, _is_diff_highlighting_enabled() 1286 | `config.rs:52,106`, `SettingsDialog.tsx`, `ProviderPanel.tsx` | L | should |
| API Keys UI visual indicators (hints, doc links, masking) | partial | SettingsWindow _create_api_keys_tab placeholders + doc links | `src/features/correction/SettingsDialog.tsx:104-136` | S | should |
| Clipboard read error handling in hotkey flow | partial | main_corrector.py:1140-1151 _show_gui_with_error() | `src-tauri/src/modules/hotkey.rs:32-37`, frontend handler | M | should |
| Style selection UI display labels (Polish) | partial | main_window.py:536 ['Normalny','Profesjonalny',...] + style_map | `src/features/correction/types.ts:58-66`, `CorrectionView.tsx:64-74` | M | should |
| Paste text button (📋 Wklej tekst) | partial | paste_button 1030-1039 paste_and_process() | `src/features/correction/CorrectionView.tsx:75-84` | S | should |
| Status label / dynamic processing messages | partial | status_label 787-794, update_status() 1108-1111 | `src/App.tsx:15-16`, `store.ts`, `CorrectionView.tsx` | S | should |
| Focus + topmost/fade-in on window show | partial | show_window() 2122-2128 topmost 100ms, alpha fade-in | `src-tauri/src/modules/tray.rs:46-48`, `hotkey.rs:46-49` | S | should |
| Tray icon notifications | partial | pystray notify() for autostart + minimize feedback | `src-tauri/src/modules/tray.rs`, `lib.rs:20` (notification plugin) | M | should |
| Tray context menu (Settings, Autostart toggle) | partial | pystray menu 2490-2498 multi-item | `src-tauri/src/modules/tray.rs:10-41` | M | should |
| Messagebox/dialog error reporting | partial | messagebox.showinfo/showerror clipboard/empty/save | React frontend dialog/toast layer | S | should |
| Console + file logging output | partial | logger.py:39-43 console + file handlers | `src-tauri/src/modules/logging.rs:24` | S | should |
| Anti-duplication guard (per session/idx) | missing | result_update_guard 449,1620-1623 | `src/features/correction/store.ts` | M | nice |
| Model variant fallback (GPT-5 naming) | missing | openai_client.py:157-163 variant list fallback | `src-tauri/crates/core/src/ai/openai.rs` | S | nice |
| API counter label (🤖 API: X/4) | partial | api_counter_label 810-815, live increment 1687 | `src/features/correction/CorrectionView.tsx:63`, `store.ts` | S | nice |
| Text accumulation dedup (Gemini _StreamingState) | partial | gemini_client.py:44-78 snapshot/delta dedup | `src-tauri/crates/core/src/ai/mod.rs:153`, `gemini.rs` | M | nice |
| Animated progress bar (0-100% cycling) | missing/partial | progress bar 1572-1586 cycles while thread alive | `src/features/correction/ProviderPanel.tsx` header, `store.ts` | M | nice |
| Thinking config (Gemini Flash/Lite/Pro budgets) | missing | gemini_client.py:86-100 thinking_budget 0/128 | `src-tauri/crates/core/src/ai/gemini.rs` | S | nice |
| ClipboardProcessingDelayMs setting | missing | runtime app.settings, off/disabled/numeric, default 400 | `config.rs`, `SettingsDialog.tsx`, `clipboard.rs:17` | M | nice |
| Minimize-to-tray button (explicit) | missing | minimize_button 1041-1048 | `src/App.tsx`/`CorrectionView.tsx`, `tray.rs` | L | nice |
| GIF animation cleanup on minimize | missing | _complete_minimize_to_tray() 2074-2077 loader.cleanup() | n/a (no GIF layer in web frontend) — likely N/A | S | nice |
| Admin rights check | missing | config_manager.py 337-345 IsUserAnAdmin() | new `autostart.rs` (only if elevation needed) | S | nice |
| DPI scaling detection | missing | _detect_monitor_scale() 102-167 / calculate_scale_factor() | CSS/Tauri scale handling, `Styles.css` | M | nice |
| Responsive window sizing/scaling | partial | setup_responsive_window() 529-552 | CSS media queries, `tauri.conf.json` | L | nice |

> Rows describing the **same feature from two audit lenses** (collapse into one task):
> `HighlightDiffs setting` ↔ `Diff highlighting render`; `AutoStartup setting` ↔ `Autostart
> registry`; `Tray context menu` ↔ `Tray notifications`; `Animated progress bar`
> (ai-streaming) ↔ `Animated progress bar` (ui-panels-diff); `Per-API cancel` ↔ `Cancel
> button per panel (✖)`; `ReasoningEffort/Verbosity config` ↔ `reasoning_effort/verbosity
> param`.

## 3. Recommended implementation order — parallel workstreams

The eight workstreams below have **no shared state** and can be assigned to different
developers/agents in parallel. Within each, steps are ordered.

### WS-A · GPT-5/o1 Responses API (the only `must`+`missing`) — highest risk, start first
Backend-only, isolated to OpenAI client.
1. In `openai.rs` add model detection: `gpt-5*` / `o1*` → Responses API path; else keep Chat Completions.
2. Implement non-streaming `responses.create` request/response shapes (input, instructions, reasoning, text).
3. Implement Responses **streaming** SSE event parsing (`response.output_text.delta`), feed existing `on_chunk`.
4. Add `reasoning.effort` and `text.verbosity` to the payload (consumes WS-B config).
5. Implement **model-variant fallback** list `[gpt-5-mini, gpt-5-mini-<date>, o1-mini, gpt-4o-mini]` retried on API error (nice, but cheap once routing exists).
6. Route selection in `crates/core/src/ai/mod.rs` + `src/modules/ai.rs`.

### WS-B · reasoning_effort / verbosity config (must pair with WS-A)
1. Add `AiSettings { reasoning_effort: String, verbosity: String }` to `config.rs` with defaults `"high"` / `"medium"`; include in load/persist + migration.
2. Add two dropdowns to `SettingsDialog.tsx` (minimal/low/medium/high; low/medium/high).
3. Thread values from `start_correction` → request → WS-A payload.

### WS-C · Streaming for Anthropic / Gemini / DeepSeek — **see §4 conflict before building**
Per provider, each independent:
1. Flip `supports_streaming()` (`types.rs:65`) to true for the provider once verified.
2. Anthropic: `messages?stream=true`, parse `content_block_delta.delta.text`, honor `CancellationToken` per chunk (parity with cancel passing already done).
3. Gemini: `streamGenerateContent`, parse incremental `candidates[].content.parts[].text`; port `_StreamingState` snapshot/delta dedup (covers the `nice` dedup row).
4. DeepSeek: SSE `data:` lines, `choices[0].delta.content`.
5. Confirm **no retry on half-stream** (matches existing OpenAI design) and stale-session guard still applies.

### WS-D · Per-provider cancel + cancel-all UI feedback (must finish partials)
1. Backend: replace single `CancellationToken` with `Vec<CancellationToken>` (one per provider) in `src/modules/ai.rs`; add `cancel_single(idx)` command keeping `cancel_session()`.
2. On cancel, emit per-panel `provider-error`/`provider-cancelled` so store sets "❌ Anulowano", resets state (currently relies on silent stale-drop).
3. `ProviderPanel.tsx`: add per-panel ✖ button (enabled while loading/streaming) → `cancel_single`.
4. `store.ts`: handle cancelled state; reset progress.
5. Add **anti-duplication guard** `(sessionId, provider)` set in store to drop duplicate `applyResult` (nice).

### WS-E · Panel UI polish — diff highlighting, loader, progress bar, status, counters, action menu, original-text view
Pure frontend (+ small backend command for reprocess).
1. **Diff highlighting**: difflib-equivalent (e.g. `diff-match-patch` or token SequenceMatcher) in a util; render changed/inserted words with red underline; gate on `highlightDiffs` setting (covers both HighlightDiffs rows). Add toggle to `SettingsDialog.tsx`.
2. **Loader/spinner** in `ProviderPanel` loading state (CSS spinner replaces GIF).
3. **Animated progress bar** in panel header driven by store status (collapses both progress-bar rows).
4. **Status label**: add `status` to store; show dynamic "📝 Przetwarzanie… (N znaków)" in `App.tsx`/`CorrectionView`.
5. **API counter X/4**: derive from store completed/errored panels; render in `correction__bar`.
6. **Original-text modal**: new component, read-only textarea + copy, bound to `store.originalText`; add 📄 button.
7. **Per-panel ⚙️ action menu** (professional / translate EN / translate PL) → backend reprocess command reusing single-provider correction with overridden style (depends on WS-D's per-provider plumbing).
8. **Style display labels**: map raw keys → Polish names in `types.ts`/dropdown.
9. **Paste-text button** + **messagebox/toast** error layer for clipboard/empty/save.
10. **API-keys UI**: placeholders, doc links, keep password masking.

### WS-F · Clipboard / hotkey robustness (must finish partials)
Backend-only.
1. `hotkey.rs`/`clipboard.rs`: wrap Ctrl+C in **3-attempt retry** with 40-80ms backoff + clipboard-change verification before the 400ms settle.
2. **Cancel in-flight corrections on window close** (`lib.rs` CloseRequested → call `cancel_session()` before hide).
3. **Clipboard-read error path**: on empty/failed read, show window + emit guidance event (frontend shows message) instead of silent `hotkey-empty`.
4. **Fallback hotkeys**: attempt to register alternatives if primary registration fails (investigate Tauri `global_shortcut` multi-register; if unsupported, register all and route).
5. **Focus/topmost**: set always-on-top briefly (100ms) on show in `tray.rs`/`hotkey.rs`.

### WS-G · System tray + autostart + notifications
Mostly backend (Windows-specific for registry).
1. New `autostart.rs`: implement registry Run-key add/remove/is_in_startup (Windows); wire `GeneralSettings.autostartup` to it on save; add toggle in `SettingsDialog.tsx`. (Admin check only if write fails.)
2. Expand `tray.rs` menu: add "Ustawienia" and dynamic "Włącz/Wyłącz autostart"; emit notifications via the already-registered notification plugin for autostart toggle + minimize-to-tray.
3. **Minimize-to-tray button** in UI.
4. **ClipboardProcessingDelayMs** setting (off/numeric, default 400) → `clipboard.rs` COPY_SETTLE.

### WS-H · Logging + misc (must finish partial)
1. **API error logging to file**: in `error.rs`/`ai.rs`, log status code + response body for Connection/Response/Timeout (in addition to frontend events) — closes the `must` partial.
2. **Console + file logging**: add a stdout layer alongside the file layer in `modules/logging.rs`.
3. **Retry/backoff verification**: confirm `retry.rs` MAX_RETRIES=2 + 800ms backoff matches Python SDK semantics; document the streaming-no-retry decision.
4. **Gemini thinking config** (Flash/Lite budget 0, Pro 128) in `gemini.rs` (nice).
5. **DPI / responsive sizing**: CSS media queries + verify Tauri scale; mark Tk-specific scaling as N/A.

**Suggested sequencing:** WS-A+WS-B together (must, highest risk) and WS-D+WS-F+WS-H
(must partials) first; WS-C, WS-E, WS-G (mostly `should`) in parallel after; nice-tier
items fold into whichever workstream owns the file.

## 4. Conflicts with project rules — decisions needed

1. **Streaming for non-OpenAI providers (Anthropic / Gemini / DeepSeek).**
   `types.rs:65` *explicitly* declares these providers non-streaming "in MVP", and the
   prior task guidance was **do NOT add streaming to non-OpenAI without verification**.
   However, the **Python app streamed all 4 providers**, and the mandate is **full parity,
   no "later" list**. → **DECISION REQUIRED:** override the MVP non-streaming decision and
   implement WS-C. Recommend implementing per-provider behind a verification step (test
   each API's stream contract against current model defaults before flipping
   `supports_streaming()`), since the original "verify first" caveat exists for a reason
   (half-stream + retry interaction, malformed SSE on Gemini). This reconciles "verify" with
   "full parity": verify, then ship — do not skip.

2. **Retry on streaming.** Current Rust design intentionally disables retry on streaming
   ("half-stream unsafe"). Python relied on SDK defaults. Implementing WS-C must preserve
   the no-retry-on-stream rule → **confirm** this is acceptable parity (Python's SDK retry
   only fired before first byte too). Likely fine; flag for sign-off.

3. **Platform-specific features (autostart, admin check, DPI scaling).** These are
   Windows-only (winreg, `IsUserAnAdmin`, `GetScaleFactorForMonitor`). Tauri is
   cross-platform. → **DECISION:** scope autostart/admin to Windows (`#[cfg(windows)]`) and
   treat DPI/responsive sizing as CSS-native (mark Tk scaling rows N/A rather than literal
   ports). Confirm Windows-only is acceptable for "full parity" given the Python app was
   Windows-only.

4. **GIF loader / GIF cleanup-on-minimize.** Python's `AnimatedGIF` + `loader.cleanup()`
   RAM-freeing is a Tk artifact. The web frontend has no GIF layer. → **DECISION:** satisfy
   parity with a CSS spinner (WS-E) and mark the GIF-cleanup row **N/A** (no resource to
   free) rather than porting literally.

5. **Settings stored as keychain + JSON vs Python plaintext config.ini.** Already a
   deliberate architecture improvement (done). New settings (reasoning/verbosity/autostart/
   clipboard-delay) must follow the **keychain-for-secrets / JSON-for-settings** split — do
   not regress to plaintext to "match" Python.
