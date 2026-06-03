import { Provider, STYLES, Style } from "../correction/types";

export interface SettingsForm {
  models: Record<Provider, string>;
  defaultStyle: string;
  apiKeys: Partial<Record<Provider, string>>;
}

export interface ValidationResult {
  ok: boolean;
  errors: Partial<Record<string, string>>;
}

/** Loose per-provider sanity checks for pasted API keys. */
export function validateApiKey(provider: Provider, key: string): string | null {
  const k = key.trim();
  if (k === "") return null; // empty = leave unchanged / clear
  if (k.length < 10) return "Klucz wygląda na zbyt krótki";
  switch (provider) {
    case "openai":
      return k.startsWith("sk-") ? null : "Klucz OpenAI zwykle zaczyna się od 'sk-'";
    case "anthropic":
      return k.startsWith("sk-ant-")
        ? null
        : "Klucz Anthropic zwykle zaczyna się od 'sk-ant-'";
    case "gemini":
      return k.startsWith("AIza") ? null : "Klucz Gemini zwykle zaczyna się od 'AIza'";
    case "deepseek":
      return k.startsWith("sk-") ? null : "Klucz DeepSeek zwykle zaczyna się od 'sk-'";
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

  for (const provider of Object.keys(form.apiKeys) as Provider[]) {
    const key = form.apiKeys[provider] ?? "";
    const err = validateApiKey(provider, key);
    if (err) errors[`key.${provider}`] = err;
  }

  return { ok: Object.keys(errors).length === 0, errors };
}
