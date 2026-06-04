import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CancelEvent,
  ChunkEvent,
  ErrorEvent,
  Provider,
  ResultEvent,
  SessionStartedEvent,
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
}

export interface SettingsView {
  models: Record<Provider, string | null>;
  default_style: string;
  highlight_diffs: boolean;
  autostartup: boolean;
  ai_settings: AiSettings;
  clipboard_delay_ms: number;
  keys_present: Record<string, boolean>;
}

export interface SaveSettingsPayload {
  models: Record<Provider, string | null>;
  default_style: string;
  highlight_diffs: boolean;
  autostartup: boolean;
  ai_settings: AiSettings;
  clipboard_delay_ms: number;
  api_keys: Partial<Record<Provider, string>>;
}

// --- commands ---------------------------------------------------------------

export const startCorrection = (text: string, style: string): Promise<number> =>
  invoke("start_correction", { text, style });

/** Diagnostic: write a line to the backend file log from the frontend. */
export const feLog = (message: string): void => {
  void invoke("fe_log", { message }).catch(() => {});
};

export const cancelSession = (): Promise<void> => invoke("cancel_session");

export const cancelProvider = (provider: Provider): Promise<void> =>
  invoke("cancel_provider", { provider });

export const reprocessProvider = (
  provider: Provider,
  style: string,
): Promise<void> => invoke("reprocess_provider", { provider, style });

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
