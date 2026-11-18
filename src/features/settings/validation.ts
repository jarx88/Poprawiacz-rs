import type { KeyAccount } from "../../lib/tauri";
import { Provider, STYLES, Style } from "../correction/types";

export interface SettingsForm {
  /** The model names of the route in use (direct or OpenRouter). */
  models: Record<Provider, string>;
  defaultStyle: string;
  apiKeys: Partial<Record<KeyAccount, string>>;
}

export interface ValidationResult {
  ok: boolean;
  errors: Partial<Record<string, string>>;
}

/** Loose per-account sanity checks for pasted API keys. */
export function validateApiKey(account: KeyAccount, key: string): string | null {
  const k = key.trim();
  if (k === "") return null; // empty = leave the stored key unchanged
  if (k.length < 10) return "Klucz wygląda na zbyt krótki";
  // Anthropic and OpenRouter keys also start with "sk-": they must not pass
  // for OpenAI or DeepSeek
  const plainSk = k.startsWith("sk-") && !k.startsWith("sk-ant-") && !k.startsWith("sk-or-");
  switch (account) {
    case "openai":
      return plainSk ? null : "Klucz OpenAI zwykle zaczyna się od 'sk-' (ale nie 'sk-ant-' ani 'sk-or-')";
    case "anthropic":
      return k.startsWith("sk-ant-")
        ? null
        : "Klucz Anthropic zwykle zaczyna się od 'sk-ant-'";
    case "gemini":
      // AI Studio issues `AQ.` auth keys since 2026; legacy standard keys start with `AIza`.
      return k.startsWith("AQ.") || k.startsWith("AIza")
        ? null
        : "Klucz Gemini zwykle zaczyna się od 'AQ.' (lub starszy 'AIza')";
    case "deepseek":
      return plainSk ? null : "Klucz DeepSeek zwykle zaczyna się od 'sk-' (ale nie 'sk-ant-' ani 'sk-or-')";
    case "openrouter":
      return k.startsWith("sk-or-") ? null : "Klucz OpenRouter zwykle zaczyna się od 'sk-or-'";
  }
}

export function isValidStyle(style: string): style is Style {
  return (STYLES as readonly string[]).includes(style);
}

export function validateSettings(form: SettingsForm): ValidationResult {
  const errors: Partial<Record<string, string>> = {};

  if (!isValidStyle(form.defaultStyle)) {
    errors.defaultStyle = `Nieznany styl: ${form.defaultStyle}`;
  }

  for (const provider of Object.keys(form.models) as Provider[]) {
    if (form.models[provider].trim() === "") {
      errors[`model.${provider}`] = "Model nie może być pusty";
    }
  }

  for (const account of Object.keys(form.apiKeys) as KeyAccount[]) {
    const err = validateApiKey(account, form.apiKeys[account] ?? "");
    if (err) errors[`key.${account}`] = err;
  }

  return { ok: Object.keys(errors).length === 0, errors };
}
