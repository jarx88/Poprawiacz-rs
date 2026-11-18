import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CancelEvent,
  ChunkEvent,
  ErrorEvent,
  Provider,
  ResultEvent,
  SessionStartedEvent,
  Style,
} from "../features/correction/types";

export type ReasoningLevel = "off" | "low" | "medium" | "high" | "max";

export interface ReasoningLevels {
  openai: ReasoningLevel;
  anthropic: ReasoningLevel;
  gemini: ReasoningLevel;
  deepseek: ReasoningLevel;
}

export interface AiSettings {
  verbosity: string;
  reasoning_levels: ReasoningLevels;
  /** User-defined system prompt; empty = built-in default. */
  system_prompt: string;
}

/** A keychain account: a provider's own key, or the one OpenRouter key. */
export type KeyAccount = Provider | "openrouter";

export interface SettingsView {
  models: Record<Provider, string | null>;
  /** One OpenRouter key serves every panel instead of the providers' own keys. */
  openrouter: boolean;
  /** Each panel's OpenRouter model id (defaults filled in). */
  openrouter_models: Record<Provider, string | null>;
  default_style: string;
  highlight_diffs: boolean;
  autostartup: boolean;
  ai_settings: AiSettings;
  clipboard_delay_ms: number;
  keys_present: Record<string, boolean>;
  default_system_prompt: string;
}

export interface SaveSettingsPayload {
  models: Record<Provider, string | null>;
  openrouter: boolean;
  openrouter_models: Record<Provider, string | null>;
  default_style: string;
  highlight_diffs: boolean;
  autostartup: boolean;
  ai_settings: AiSettings;
  clipboard_delay_ms: number;
  api_keys: Partial<Record<KeyAccount, string>>;
}

// --- commands ---------------------------------------------------------------

export const startCorrection = (text: string, style: string): Promise<number> =>
  invoke("start_correction", { text, style });

/** Diagnostic: write a line to the backend file log from the frontend. */
export const feLog = (message: string): void => {
  void invoke("fe_log", { message }).catch(() => {});
};

export const cancelSession = (): Promise<void> => invoke("cancel_session");

export interface BuildInfo {
  version: string;
  build: string;
  /** Unix seconds; a newer build has a later time. */
  built_at: number;
}

export const getAppBuild = (): Promise<BuildInfo> => invoke("app_build");

export const cancelProvider = (provider: Provider): Promise<void> =>
  invoke("cancel_provider", { provider });

export const reprocessProvider = (
  sessionId: number,
  provider: Provider,
  style: string,
): Promise<void> => invoke("reprocess_provider", { sessionId, provider, style });

export const rememberCorrection = (
  original: string,
  corrected: string,
  style: Style,
): Promise<number> => invoke("remember_correction", { original, corrected, style });

export const readClipboard = (): Promise<string> => invoke("read_clipboard");

export const writeClipboard = (text: string): Promise<void> =>
  invoke("write_clipboard", { text });

export const pasteText = (text: string): Promise<void> =>
  invoke("paste_text", { text });

export const getSettings = (): Promise<SettingsView> => invoke("get_settings");

export const saveSettings = (payload: SaveSettingsPayload): Promise<void> =>
  invoke("save_settings", { payload });

export const migrateConfigIni = (path: string): Promise<number> =>
  invoke("migrate_config_ini", { path });

/** The provider's current model names. `apiKey` is a key typed into the dialog
 *  but not saved yet; without it the backend uses the stored one. */
export const listModels = (provider: Provider, apiKey?: string): Promise<string[]> =>
  invoke("list_models", { provider, apiKey });

/** The text models OpenRouter offers for each panel (its list is public). */
export const listOpenrouterModels = (): Promise<Record<Provider, string[]>> =>
  invoke("list_openrouter_models");

/** Open a web page in the system browser (the opener plugin's command). */
export const openUrl = (url: string): Promise<void> =>
  invoke("plugin:opener|open_url", { url });

// --- events -----------------------------------------------------------------

export const onSessionStarted = (
  cb: (e: SessionStartedEvent) => void,
): Promise<UnlistenFn> =>
  listen<SessionStartedEvent>("session-started", (e) => cb(e.payload));

export const onChunk = (cb: (e: ChunkEvent) => void): Promise<UnlistenFn> =>
  listen<ChunkEvent>("provider-chunk", (e) => cb(e.payload));

export const onResult = (cb: (e: ResultEvent) => void): Promise<UnlistenFn> =>
  listen<ResultEvent>("provider-result", (e) => cb(e.payload));

export const onError = (cb: (e: ErrorEvent) => void): Promise<UnlistenFn> =>
  listen<ErrorEvent>("provider-error", (e) => cb(e.payload));

export const onCancelled = (cb: (e: CancelEvent) => void): Promise<UnlistenFn> =>
  listen<CancelEvent>("provider-cancelled", (e) => cb(e.payload));

export const onRestarted = (cb: (e: CancelEvent) => void): Promise<UnlistenFn> =>
  listen<CancelEvent>("provider-restarted", (e) => cb(e.payload));

export const onHotkeyEmpty = (cb: () => void): Promise<UnlistenFn> =>
  listen("hotkey-empty", () => cb());

export const onOpenSettings = (cb: () => void): Promise<UnlistenFn> =>
  listen("open-settings", () => cb());

/** Open the settings dialog from anywhere in the UI (same event the tray sends). */
export const openSettings = (): Promise<void> => emit("open-settings");
