import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ChunkEvent,
  ErrorEvent,
  Provider,
  ResultEvent,
  SessionStartedEvent,
} from "../features/correction/types";

export interface SettingsView {
  models: Record<Provider, string | null>;
  default_style: string;
  keys_present: Record<string, boolean>;
}

export interface SaveSettingsPayload {
  models: Record<Provider, string | null>;
  default_style: string;
  api_keys: Partial<Record<Provider, string>>;
}

// --- commands ---------------------------------------------------------------

export const startCorrection = (text: string, style: string): Promise<number> =>
  invoke("start_correction", { text, style });

export const cancelSession = (): Promise<void> => invoke("cancel_session");

export const readClipboard = (): Promise<string> => invoke("read_clipboard");

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

export const onHotkeyEmpty = (cb: () => void): Promise<UnlistenFn> =>
  listen("hotkey-empty", () => cb());
