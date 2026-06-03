# PoprawiaczTekstu (Tauri 2 + Rust + React)

Windows-first desktop app for AI text correction. Select text anywhere, press
**`Ctrl+Shift+C`**, and OpenAI, Anthropic, Gemini and DeepSeek correct it
concurrently in four panels — pick the best version and paste it back.

Greenfield rewrite of the Python/CustomTkinter `PoprawiaczTekstuPy`, modeled
architecturally after [Terax](https://github.com/crynta/terax-ai): Rust owns
native OS / network / secrets, React owns presentation, the boundary is typed
Tauri commands + events.

## Architecture

```
src/                         React 19 + TypeScript frontend
  features/correction/       4-provider session store (session-isolated) + panels
  features/settings/         settings dialog (lazy) + validation
  lib/tauri.ts               typed command/event wrappers
src-tauri/
  src/                       poprawiacz app crate (Tauri commands, hotkey, clipboard)
  crates/core/               poprawiacz-core — pure logic, NO tauri dep, tested everywhere
    src/ai/                  per-provider request/response, retry, SSE, timeouts
    src/{config,prompts,logging}.rs
```

**Why the split:** all provider mapping, retry/backoff, timeouts, INI parsing
and prompts live in `poprawiacz-core`, a crate with no GUI/Tauri dependency. It
compiles and tests on any platform (incl. headless CI). The thin app crate adds
the desktop integration.

## Key behavior preserved from the Python app

- Hotkey `Ctrl+Shift+C` → copy selection → 4-way concurrent correction → paste.
- Per-provider error isolation; one failure never blocks the others.
- Monotonic session id + cancellation: stale results never overwrite a newer run.
- Timeouts: 25 s standard, **35 s DeepSeek**, 8 s connect, 2 retries.
- OpenAI streams; the other three return whole responses (MVP).
- 7 prompt styles (normal / professional / translate_en / translate_pl /
  change_meaning / summary / prompt), byte-for-byte from `utils/prompts.py`.
- API keys move from `config.ini` into the **OS keychain** (`keyring`).

## Quick start

```bash
pnpm install
pnpm tauri dev
```

See [docs/windows-build.md](docs/windows-build.md) for prerequisites, the Windows
smoke checklist, and quality-gate commands. See
[docs/implementation-plan.md](docs/implementation-plan.md) for the full migration
plan and Python→Rust feature mapping.

## Quality gates

```bash
pnpm exec tsc --noEmit
pnpm test
cd src-tauri && cargo test --workspace --locked
cd src-tauri && cargo clippy --workspace --all-targets --locked -- -D warnings
```
