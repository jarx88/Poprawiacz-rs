import { useEffect, useState } from "react";
import {
  getSettings,
  migrateConfigIni,
  saveSettings,
} from "../../lib/tauri";
import { PROVIDERS, PROVIDER_META, Provider, STYLES } from "../correction/types";
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

export default function SettingsDialog({ onClose }: Props) {
  const [models, setModels] = useState<Record<Provider, string>>(emptyModels());
  const [defaultStyle, setDefaultStyle] = useState("normal");
  const [keysPresent, setKeysPresent] = useState<Record<string, boolean>>({});
  const [apiKeys, setApiKeys] = useState<Partial<Record<Provider, string>>>({});
  const [errors, setErrors] = useState<Record<string, string | undefined>>({});
  const [status, setStatus] = useState<string>("");

  useEffect(() => {
    getSettings()
      .then((s) => {
        setModels({
          openai: s.models.openai ?? "",
          anthropic: s.models.anthropic ?? "",
          gemini: s.models.gemini ?? "",
          deepseek: s.models.deepseek ?? "",
        });
        setDefaultStyle(s.default_style || "normal");
        setKeysPresent(s.keys_present ?? {});
      })
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
        api_keys: apiKeys,
      });
      setStatus("Zapisano ✅");
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
      setStatus(`Zmigrowano ${n} kluczy. Wczytuję ponownie…`);
      const s = await getSettings();
      setModels({
        openai: s.models.openai ?? "",
        anthropic: s.models.anthropic ?? "",
        gemini: s.models.gemini ?? "",
        deepseek: s.models.deepseek ?? "",
      });
      setDefaultStyle(s.default_style || "normal");
      setKeysPresent(s.keys_present ?? {});
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
          <select
            value={defaultStyle}
            onChange={(e) => setDefaultStyle(e.target.value)}
          >
            {STYLES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
          {errors.defaultStyle && (
            <span className="field__err">{errors.defaultStyle}</span>
          )}
        </label>

        {PROVIDERS.map((p) => (
          <fieldset key={p} className="provider-settings">
            <legend style={{ color: PROVIDER_META[p].color }}>
              {PROVIDER_META[p].label}
            </legend>
            <label className="field">
              Model
              <input
                value={models[p]}
                onChange={(e) =>
                  setModels((m) => ({ ...m, [p]: e.target.value }))
                }
              />
              {errors[`model.${p}`] && (
                <span className="field__err">{errors[`model.${p}`]}</span>
              )}
            </label>
            <label className="field">
              Klucz API{" "}
              {keysPresent[p] ? "(zapisany — zostaw puste, by nie zmieniać)" : "(brak)"}
              <input
                type="password"
                placeholder={keysPresent[p] ? "••••••••" : "wklej klucz"}
                value={apiKeys[p] ?? ""}
                onChange={(e) =>
                  setApiKeys((k) => ({ ...k, [p]: e.target.value }))
                }
              />
              {errors[`key.${p}`] && (
                <span className="field__err">{errors[`key.${p}`]}</span>
              )}
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
