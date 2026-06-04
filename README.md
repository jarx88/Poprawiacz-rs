# PoprawiaczTekstu (Tauri 2 + Rust + React)

Modern AI-powered text correction tool. Select text anywhere, press **`Ctrl+Shift+C`**,
and OpenAI, Anthropic, Gemini and DeepSeek correct it concurrently in four panels —
pick the best version and paste it back.

![build](https://github.com/jarx88/Poprawiacz-rs/actions/workflows/build.yml/badge.svg)

**Greenfield rewrite** of the Python/CustomTkinter [PoprawiaczTekstuPy](https://github.com/jarx88/PoprawiaczTekstuPy)
and the egui-based [poprawiacz-tekstu-rs](https://github.com/jarx88/poprawiacz-tekstu-rs),
this time on **Tauri 2** — Rust owns native OS / network / secrets, React owns
presentation, the boundary is typed Tauri commands + events. Architecturally modeled
after [Terax](https://github.com/crynta/terax-ai).

## ✨ Features

- 🦀 **Rust core** — provider mapping, retry/backoff, timeouts, INI parsing and prompts live in a GUI-free crate, tested on any platform (incl. headless CI)
- 🎨 **Modern UI** — React 19 + TypeScript, frameless custom titlebar, 4-panel layout
- ⚡ **Global hotkey** — `Ctrl+Shift+C` auto-copies the selected text and starts correction
- 🤖 **4 AI providers** — OpenAI, Anthropic, Gemini, DeepSeek running concurrently
- 📋 **Auto-paste** — pick a panel and paste the chosen correction back into the source app
- 🌊 **Streaming** — real-time token streaming from OpenAI (others return whole responses)
- ⚙️ **Cancellation** — a new hotkey cancels the previous run; stale results never overwrite a newer one
- 🎨 **Color-coded panels** — each provider has its own accent color
- 🔍 **Word-level diff** — see exactly what each model changed vs. the original
- ✍️ **7 prompt styles** — normal / professional / translate_en / translate_pl / change_meaning / summary / prompt (byte-for-byte from the Python app)
- 🔐 **OS keychain** — API keys live in the system keychain (`keyring`), never in a plaintext config file
- 🪟 **System tray** — minimize to tray with show/quit menu
- 🚀 **Autostart** — optional launch on Windows login

## 🏗️ Architecture

```
src/                         React 19 + TypeScript frontend
  features/correction/       4-provider session store (session-isolated) + panels + diff
  features/settings/         settings dialog + validation
  lib/tauri.ts               typed command/event wrappers
  Titlebar.tsx               custom frameless window chrome
src-tauri/
  src/modules/               poprawiacz app crate — Tauri commands, hotkey, clipboard, tray, autostart
  crates/core/               poprawiacz-core — pure logic, NO tauri dep, tested everywhere
    src/ai/                  per-provider request/response, retry, SSE, timeouts
    src/{config,prompts,logging}.rs
```

**Why the split:** all provider logic compiles and tests on any platform without a
GUI. The thin app crate adds the desktop integration (windowing, global shortcut,
clipboard, tray, secrets). This keeps the testable surface large and platform-agnostic.

## 🚀 Installation

### Option 1: Download the portable build (recommended)

1. Go to the [Actions](https://github.com/jarx88/Poprawiacz-rs/actions) tab
2. Open the latest successful **build** run
3. Download the **`PoprawiaczTekstu-portable-windows`** artifact
4. Unzip and run `poprawiacz.exe` — no installer, WebView2 is preinstalled on Windows 10/11

> SmartScreen warning: click **More info → Run anyway**. The binary is unsigned, not unsafe.

### Option 2: Build from source

**Requirements:**
- [Rust](https://rustup.rs/) (stable, edition 2021, `rust-version = 1.77`)
- [Node.js](https://nodejs.org/) 22+ and [pnpm](https://pnpm.io/) 11+
- Linux only — Tauri/WebKit system deps:
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev \
    libayatana-appindicator3-dev
  ```

**Run / build:**
```bash
git clone https://github.com/jarx88/Poprawiacz-rs.git
cd Poprawiacz-rs
pnpm install
pnpm tauri dev      # development
pnpm tauri build    # release build → src-tauri/target/release/poprawiacz(.exe)
```

See [docs/windows-build.md](docs/windows-build.md) for the Windows smoke checklist
and [docs/implementation-plan.md](docs/implementation-plan.md) for the full
Python→Rust feature mapping.

## ⚙️ Configuration

### First run

1. Launch the app
2. Open **Settings**
3. Paste your API keys (stored in the **OS keychain**, not on disk):
   - **OpenAI** — `sk-...` from <https://platform.openai.com/api-keys>
   - **Anthropic** — `sk-ant-...` from <https://console.anthropic.com/>
   - **Gemini** — `AIza...` from <https://aistudio.google.com/app/apikey>
   - **DeepSeek** — `sk-...` from <https://platform.deepseek.com/api_keys>
4. Pick models (or keep the defaults) and **Save**

### Where things are stored

- **API keys** → system keychain (Windows Credential Manager / macOS Keychain / Secret Service on Linux) via [`keyring`](https://github.com/hwchen/keyring-rs)
- **Models, prompts, preferences** → INI config managed by the core crate (`config.ini`)

## 🎯 Usage

1. **Select text** in any application
2. **Press `Ctrl+Shift+C`** — the app auto-copies the selection
3. **Watch the 4 panels** — each provider streams/returns its correction independently
4. **Click the best result** — the panel highlights
5. **Paste** — the chosen correction goes back into the source app

Per-provider error isolation means one failed provider never blocks the other three.

### Hotkeys

- **`Ctrl+Shift+C`** — capture selection and run all providers
- **Cancel** — stop the in-flight run
- **Minimize to tray** — hide the window; reopen from the tray menu

## 🔧 Development

### Quality gates

```bash
pnpm exec tsc --noEmit
pnpm test                                                              # 20 frontend tests
cd src-tauri && cargo test --workspace --locked                       # 60 Rust tests
cd src-tauri && cargo clippy --workspace --all-targets --locked -- -D warnings
```

These are exactly the checks the CI runs.

### Key behavior preserved from the Python app

- Hotkey `Ctrl+Shift+C` → copy selection → 4-way concurrent correction → paste
- Per-provider error isolation; one failure never blocks the others
- Monotonic session id + cancellation: stale results never overwrite a newer run
- Timeouts: **25 s** standard, **35 s DeepSeek**, **8 s** connect, **2** retries
- OpenAI streams; the other three return whole responses
- 7 prompt styles, byte-for-byte from `utils/prompts.py`
- API keys moved from `config.ini` into the **OS keychain**

### CI/CD

GitHub Actions ([.github/workflows/build.yml](.github/workflows/build.yml)) runs on every
push to `main`:

- **checks** (Linux) — typecheck, frontend tests, Rust tests, clippy with `-D warnings`
- **windows-portable** — builds a single portable `poprawiacz.exe` and uploads it as an artifact

## 📦 Technologies

- **Shell**: [Tauri 2](https://tauri.app/) (Rust backend + WebView frontend)
- **Frontend**: [React 19](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/) + [Zustand](https://github.com/pmndrs/zustand) + [Vite](https://vitejs.dev/)
- **Async runtime**: [tokio](https://tokio.rs/)
- **HTTP**: [reqwest](https://github.com/seanmonstar/reqwest) (rustls, streaming)
- **Secrets**: [keyring](https://github.com/hwchen/keyring-rs) (OS keychain)
- **Keyboard simulation**: [enigo](https://github.com/enigo-rs/enigo) (cross-platform)
- **Tauri plugins**: global-shortcut, clipboard-manager, notification, store, window-state, opener
- **Config**: [configparser](https://github.com/QEDK/configparser-rs) (INI)
- **Logging**: [tracing](https://github.com/tokio-rs/tracing)

## 🐛 Troubleshooting

**Hotkey not working** — another app may already own `Ctrl+Shift+C` (screenshot tools are common offenders). Close the conflicting app.

**SmartScreen warning (Windows)** — click **More info → Run anyway**. The build is unsigned, not malicious.

**API errors** — verify the keys in Settings and check your internet connection.

**Linux build fails** — install the WebKit/Tauri system deps listed under [Build from source](#option-2-build-from-source).

## ❓ FAQ

**Q: Why Tauri instead of egui (the previous Rust attempt)?**
A: A real HTML/CSS UI gives a far better-looking, frameless 4-panel layout and word-level diff than immediate-mode egui, while Rust still owns all the native/network/secret work.

**Q: Can I use it without API keys?**
A: No — each panel needs the corresponding provider key.

**Q: Does it work offline?**
A: No, it calls the provider APIs over the network.

**Q: macOS support?**
A: The core crate is platform-agnostic and `keyring`/`enigo` support macOS, but the app is Windows-first and not actively tested on macOS yet.

## 📄 License

MIT — the `poprawiacz-core` crate is published under the MIT license.

## 🙏 Acknowledgments

- Python original: [PoprawiaczTekstuPy](https://github.com/jarx88/PoprawiaczTekstuPy)
- First Rust attempt (egui): [poprawiacz-tekstu-rs](https://github.com/jarx88/poprawiacz-tekstu-rs)
- Architecture inspiration: [Terax](https://github.com/crynta/terax-ai)
- AI providers: OpenAI, Anthropic, Google, DeepSeek

---

**Built with ❤️ and 🦀 Rust + Tauri**
