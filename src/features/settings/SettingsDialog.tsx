import { useEffect, useState } from "react";
import {
  AiSettings,
  getSettings,
  migrateConfigIni,
  saveSettings,
} from "../../lib/tauri";
import { PROVIDERS, PROVIDER_META, Provider, STYLES, STYLE_LABELS } from "../correction/types";
import { validateSettings } from "./validation";

interface Props {
  onClose: () => void;
}

const emptyModels = (): Record<Provider, string> => ({
  openai: "",
  anthropic: "",
  gemini: "",
  deepseek: "",
});

const KEY_HINTS: Record<Provider, string> = {
  openai: "platform.openai.com/api-keys",
  anthropic: "console.anthropic.com/settings/keys",
  gemini: "aistudio.google.com/app/apikey",
  deepseek: "platform.deepseek.com/api_keys",
};

const LEVELS = ["off", "low", "medium", "high", "max"] as const;
const LEVEL_LABELS: Record<string, string> = {
  off: "Off (najszybciej)",
  low: "Low",
  medium: "Medium",
  high: "High",
  max: "Max",
};
const VERBOSITIES = ["low", "medium", "high"];

export default function SettingsDialog({ onClose }: Props) {
  const [models, setModels] = useState<Record<Provider, string>>(emptyModels());
  const [defaultStyle, setDefaultStyle] = useState("normal");
  const [highlightDiffs, setHighlightDiffs] = useState(false);
  const [autostartup, setAutostartup] = useState(false);
  const [clipboardDelayMs, setClipboardDelayMs] = useState(400);
  const [ai, setAi] = useState<AiSettings>({
    verbosity: "medium",
    reasoning_levels: { openai: "off", anthropic: "off", gemini: "off", deepseek: "off" },
  });
  const [keysPresent, setKeysPresent] = useState<Record<string, boolean>>({});
  const [apiKeys, setApiKeys] = useState<Partial<Record<Provider, string>>>({});
  const [errors, setErrors] = useState<Record<string, string | undefined>>({});
  const [status, setStatus] = useState<string>("");

  const applyView = (s: Awaited<ReturnType<typeof getSettings>>) => {
    setModels({
      openai: s.models.openai ?? "",
      anthropic: s.models.anthropic ?? "",
      gemini: s.models.gemini ?? "",
      deepseek: s.models.deepseek ?? "",
    });
    setDefaultStyle(s.default_style || "normal");
    setHighlightDiffs(s.highlight_diffs);
    setAutostartup(s.autostartup);
    setClipboardDelayMs(s.clipboard_delay_ms);
    setAi(s.ai_settings);
    setKeysPresent(s.keys_present ?? {});
  };

  useEffect(() => {
    getSettings()
      .then(applyView)
      .catch((e) => setStatus(`Nie udało się wczytać ustawień: ${e}`));
  }, []);

  const save = async () => {
    const result = validateSettings({ models, defaultStyle, apiKeys });
    setErrors(result.errors);
    if (!result.ok) return;
    try {
      await saveSettings({
        models,
        default_style: defaultStyle,
        highlight_diffs: highlightDiffs,
        autostartup,
        ai_settings: ai,
        clipboard_delay_ms: clipboardDelayMs,
        api_keys: apiKeys,
      });
      onClose();
    } catch (e) {
      setStatus(`Błąd zapisu: ${e}`);
    }
  };

  const migrate = async () => {
    const path = prompt("Ścieżka do starego config.ini:");
    if (!path) return;
    try {
      const n = await migrateConfigIni(path);
      setStatus(`Zmigrowano ${n} kluczy.`);
      applyView(await getSettings());
    } catch (e) {
      setStatus(`Migracja nieudana: ${e}`);
    }
  };

  return (
    <div className="modal" role="dialog" aria-label="Ustawienia">
      <div className="modal__card">
        <h2>⚙️ Ustawienia</h2>

        <label className="field">
          Domyślny styl
          <select value={defaultStyle} onChange={(e) => setDefaultStyle(e.target.value)}>
            {STYLES.map((s) => (
              <option key={s} value={s}>
                {STYLE_LABELS[s]}
              </option>
            ))}
          </select>
          {errors.defaultStyle && <span className="field__err">{errors.defaultStyle}</span>}
        </label>

        <label className="field field--row">
          <input
            type="checkbox"
            checked={highlightDiffs}
            onChange={(e) => setHighlightDiffs(e.target.checked)}
          />
          Podświetlaj zmiany (diff)
        </label>

        <label className="field field--row">
          <input
            type="checkbox"
            checked={autostartup}
            onChange={(e) => setAutostartup(e.target.checked)}
          />
          Uruchamiaj przy starcie systemu (Windows)
        </label>

        <label className="field">
          Opóźnienie schowka (ms)
          <input
            type="number"
            min={0}
            value={clipboardDelayMs}
            onChange={(e) => setClipboardDelayMs(Number(e.target.value))}
          />
        </label>

        <fieldset className="provider-settings">
          <legend>OpenAI — długość odpowiedzi (Responses API)</legend>
          <label className="field">
            Verbosity
            <select
              value={ai.verbosity}
              onChange={(e) => setAi((a) => ({ ...a, verbosity: e.target.value }))}
            >
              {VERBOSITIES.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </label>
        </fieldset>

        {PROVIDERS.map((p) => (
          <fieldset key={p} className="provider-settings">
            <legend style={{ color: PROVIDER_META[p].color }}>{PROVIDER_META[p].label}</legend>
            <label className="field">
              Model
              <input
                value={models[p]}
                onChange={(e) => setModels((m) => ({ ...m, [p]: e.target.value }))}
              />
              {errors[`model.${p}`] && <span className="field__err">{errors[`model.${p}`]}</span>}
            </label>
            <label className="field">
              Klucz API {keysPresent[p] ? "(zapisany — zostaw puste, by nie zmieniać)" : "(brak)"}
              <input
                type="password"
                placeholder={keysPresent[p] ? "••••••••" : "wklej klucz"}
                value={apiKeys[p] ?? ""}
                onChange={(e) => setApiKeys((k) => ({ ...k, [p]: e.target.value }))}
              />
              <span className="field__hint">🔗 {KEY_HINTS[p]}</span>
              {errors[`key.${p}`] && <span className="field__err">{errors[`key.${p}`]}</span>}
            </label>
            <label className="field">
              Moc rozumowania
              <select
                value={ai.reasoning_levels[p]}
                onChange={(e) =>
                  setAi((a) => ({
                    ...a,
                    reasoning_levels: { ...a.reasoning_levels, [p]: e.target.value },
                  }))
                }
              >
                {LEVELS.map((v) => (
                  <option key={v} value={v}>
                    {LEVEL_LABELS[v]}
                  </option>
                ))}
              </select>
              <span className="field__hint">Off = bez myślenia, najszybciej (zalecane do korekty)</span>
            </label>
          </fieldset>
        ))}

        {status && <p className="modal__status">{status}</p>}

        <div className="modal__actions">
          <button onClick={migrate}>📥 Migruj config.ini</button>
          <span style={{ flex: 1 }} />
          <button onClick={onClose}>Anuluj</button>
          <button className="primary" onClick={save}>
            Zapisz
          </button>
        </div>
      </div>
    </div>
  );
}
