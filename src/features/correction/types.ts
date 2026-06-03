export type Provider = "openai" | "anthropic" | "gemini" | "deepseek";

export const PROVIDERS: Provider[] = [
  "openai",
  "anthropic",
  "gemini",
  "deepseek",
];

export const PROVIDER_META: Record<
  Provider,
  { label: string; color: string }
> = {
  openai: { label: "OpenAI", color: "#10a37f" },
  anthropic: { label: "Anthropic", color: "#d97706" },
  gemini: { label: "Gemini", color: "#4285f4" },
  deepseek: { label: "DeepSeek", color: "#7c3aed" },
};

export type PanelStatus =
  | "idle"
  | "loading"
  | "streaming"
  | "done"
  | "error";

export interface PanelState {
  status: PanelStatus;
  text: string;
  error: string | null;
  elapsedMs: number | null;
}

export interface SessionStartedEvent {
  session_id: number;
  text: string;
}

export interface ChunkEvent {
  session_id: number;
  provider: Provider;
  delta: string;
}

export interface ResultEvent {
  session_id: number;
  provider: Provider;
  text: string;
  elapsed_ms: number;
}

export interface ErrorEvent {
  session_id: number;
  provider: Provider;
  message: string;
}

export const STYLES = [
  "normal",
  "professional",
  "translate_en",
  "translate_pl",
  "change_meaning",
  "summary",
  "prompt",
] as const;

export type Style = (typeof STYLES)[number];
