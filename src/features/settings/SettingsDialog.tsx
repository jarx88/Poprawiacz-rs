import { useEffect, useRef, useState } from "react";
import { Icon } from "../../Icon";
import {
  AiSettings,
  getSettings,
  listModels,
  migrateConfigIni,
  openUrl,
  saveSettings,
} from "../../lib/tauri";
import { useModal } from "../../lib/useModal";
import { PROVIDERS, PROVIDER_META, Provider, STYLES, STYLE_LABELS } from "../correction/types";
import { validateApiKey, validateSettings } from "./validation";

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

/** Where each provider publishes its model names. */
const MODEL_DOCS: Record<Provider, string> = {
  openai: "https://platform.openai.com/docs/models",
  anthropic: "https://docs.anthropic.com/en/docs/about-claude/models",
  gemini: "https://ai.google.dev/gemini-api/docs/models",
  deepseek: "https://api-docs.deepseek.com/quick_start/pricing",
};
/** The select's last option: switch this provider to a typed model name. */
const MANUAL = "\u0000manual";

const LEVELS = ["off", "low", "medium", "high", "max"] as const;
const LEVEL_LABELS: Record<string, string> = {
  off: "Off (najszybciej)",
  low: "Low",
  medium: "Medium",
  high: "High",
  max: "Max",
};
const VERBOSITIES = ["low", "medium", "high"];

/** The model name: a choice from the provider's list once it is fetched, a text
 *  field until then, without a key, or after "Wpisz ręcznie…". */
function ModelField(props: {
  value: string;
  /** The name the dialog opened with: stays selectable even when the provider
   *  no longer lists it. */
  saved: string;
  list: string[] | string | undefined;
  manual: boolean;
  onChange: (model: string) => void;
  onManual: () => void;
}) {
  const { value, saved, list, manual, onChange, onManual } = props;
  // a list that arrives while a name is being typed waits until the field is left
  const [typing, setTyping] = useState(false);
  if (manual || typing || !Array.isArray(list))
    return (
      <input
        value={value}
        autoFocus={manual}
        onFocus={() => setTyping(true)}
        onBlur={() => setTyping(false)}
        onChange={(e) => onChange(e.target.value)}
      />
    );
  const unlisted = [...new Set([saved, value])].filter(
    (name) => !list.includes(name) && (name !== "" || value === ""),
  );
  return (
    <select
      value={value}
      onChange={(e) => (e.target.value === MANUAL ? onManual() : onChange(e.target.value))}
    >
      {[...unlisted, ...list].map((id) => (
        <option key={id} value={id}>
          {id || "— wybierz model —"}
        </option>
      ))}
      <option value={MANUAL}>Wpisz ręcznie…</option>
    </select>
  );
}

export default function SettingsDialog({ onClose }: Props) {
  const [models, setModels] = useState<Record<Provider, string>>(emptyModels());
  const [defaultStyle, setDefaultStyle] = useState("normal");
  const [highlightDiffs, setHighlightDiffs] = useState(false);
  const [autostartup, setAutostartup] = useState(false);
  const [clipboardDelayMs, setClipboardDelayMs] = useState(400);
  const [ai, setAi] = useState<AiSettings>({
    verbosity: "medium",
    reasoning_levels: { openai: "off", anthropic: "off", gemini: "off", deepseek: "off" },
    system_prompt: "",
  });
  const [defaultPrompt, setDefaultPrompt] = useState("");
  const [keysPresent, setKeysPresent] = useState<Record<string, boolean>>({});
  const [apiKeys, setApiKeys] = useState<Partial<Record<Provider, string>>>({});
  const [errors, setErrors] = useState<Record<string, string | undefined>>({});
  const [status, setStatus] = useState<string>("");
  // per provider: the fetched model names, or why they could not be fetched
  const [modelLists, setModelLists] = useState<Partial<Record<Provider, string[] | string>>>({});
  const [manual, setManual] = useState<Partial<Record<Provider, boolean>>>({});
  const [savedModels, setSavedModels] = useState<Record<Provider, string>>(emptyModels());
  const { ref, close } = useModal();

  // Only the newest request of a provider may set its list: an older one
  // (stored key) can come back after a newer one (key just typed).
  const requests = useRef<Partial<Record<Provider, number>>>({});
  const typedKey = (p: Provider) => (apiKeys[p] ?? "").trim();
  const loadModels = (p: Provider, apiKey?: string) => {
    const request = (requests.current[p] = (requests.current[p] ?? 0) + 1);
    return listModels(p, apiKey)
      .then((list) => (list.length ? list : "Dostawca nie zwrócił żadnego modelu."))
      .catch((e) => String(e))
      .then((result) => {
        if (requests.current[p] === request) setModelLists((l) => ({ ...l, [p]: result }));
      });
  };

  const applyView = (s: Awaited<ReturnType<typeof getSettings>>) => {
    const loaded = {
      openai: s.models.openai ?? "",
      anthropic: s.models.anthropic ?? "",
      gemini: s.models.gemini ?? "",
      deepseek: s.models.deepseek ?? "",
    };
    setModels(loaded);
    setSavedModels(loaded);
    setDefaultStyle(s.default_style || "normal");
    setHighlightDiffs(s.highlight_diffs);
    setAutostartup(s.autostartup);
    setClipboardDelayMs(s.clipboard_delay_ms);
    setAi({ ...s.ai_settings, system_prompt: s.ai_settings.system_prompt ?? "" });
    setDefaultPrompt(s.default_system_prompt);
    setKeysPresent(s.keys_present ?? {});
    for (const p of PROVIDERS) if (s.keys_present?.[p]) void loadModels(p);
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
        // an emptied field means "leave the stored key", as the dialog says;
        // sending "" would delete it
        api_keys: Object.fromEntries(Object.entries(apiKeys).filter(([, key]) => key?.trim())),
      });
      close();
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
    <dialog ref={ref} className="modal" aria-label="Ustawienia" onClose={onClose}>
      <div className="modal__card modal__card--wide">
        <header className="modal__head">
          <h2>Ustawienia</h2>
          <button
            className="icon-btn"
            title="Zamknij bez zapisywania"
            aria-label="Zamknij bez zapisywania"
            onClick={close}
          >
            <Icon name="close" />
          </button>
        </header>

        <div className="modal__body">
          <section className="group">
            <h3>Ogólne</h3>
            <div className="group__grid">
              <label className="field">
                Domyślny styl
                <select value={defaultStyle} onChange={(e) => setDefaultStyle(e.target.value)}>
                  {STYLES.map((s) => (
                    <option key={s} value={s}>
                      {STYLE_LABELS[s]}
                    </option>
                  ))}
                </select>
                {errors.defaultStyle && (
                  <span className="field__err">{errors.defaultStyle}</span>
                )}
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
            </div>
            <label className="field field--row">
              <input
                type="checkbox"
                checked={highlightDiffs}
                onChange={(e) => setHighlightDiffs(e.target.checked)}
              />
              Podkreślaj zmienione słowa
            </label>
            <label className="field field--row">
              <input
                type="checkbox"
                checked={autostartup}
                onChange={(e) => setAutostartup(e.target.checked)}
              />
              Uruchamiaj przy starcie systemu (Windows)
            </label>
          </section>

          <section className="group">
            <h3>Prompt systemowy</h3>
            <p className="group__hint">
              Instrukcja wysyłana do AI przy stylach korekty. Style Profesjonalny i Prompt
              mają własne instrukcje. Puste pole oznacza wbudowany prompt.
            </p>
            <textarea
              rows={4}
              aria-label="Prompt systemowy"
              placeholder="Puste = wbudowany domyślny prompt"
              value={ai.system_prompt}
              onChange={(e) => setAi((a) => ({ ...a, system_prompt: e.target.value }))}
            />
            <button
              type="button"
              onClick={() => setAi((a) => ({ ...a, system_prompt: defaultPrompt }))}
            >
              Wstaw domyślny do edycji
            </button>
          </section>

          <section className="group">
            <h3>Dostawcy</h3>
            <p className="group__hint">
              Moc rozumowania Off odpowiada najszybciej i wystarcza do korekty. Puste pole
              klucza zostawia zapisany klucz bez zmian.
            </p>
            <div className="group__grid">
              {PROVIDERS.map((p) => (
                <div
                  key={p}
                  className="provider-card"
                  role="group"
                  aria-label={PROVIDER_META[p].label}
                  style={{ "--pc": PROVIDER_META[p].color } as React.CSSProperties}
                >
                  <div className="provider-card__head">
                    <span className="panel__title">
                      <span className="panel__dot" />
                      {PROVIDER_META[p].label}
                    </span>
                    <span className={`chip chip--${keysPresent[p] ? "ok" : "warn"}`}>
                      {keysPresent[p] ? "klucz zapisany" : "brak klucza"}
                    </span>
                  </div>
                  <label className="field">
                    Model
                    <ModelField
                      value={models[p]}
                      saved={savedModels[p]}
                      list={modelLists[p]}
                      manual={manual[p] ?? false}
                      onChange={(model) => setModels((m) => ({ ...m, [p]: model }))}
                      onManual={() => setManual((m) => ({ ...m, [p]: true }))}
                    />
                    {errors[`model.${p}`] && (
                      <span className="field__err">{errors[`model.${p}`]}</span>
                    )}
                    <span className="field__hint">
                      {typeof modelLists[p] === "string" && (
                        <>Lista modeli niedostępna: {modelLists[p]} </>
                      )}
                      <button
                        className="link"
                        onClick={() => openUrl(MODEL_DOCS[p]).catch((e) => setStatus(String(e)))}
                      >
                        Nazwy modeli u dostawcy
                      </button>
                      {manual[p] && Array.isArray(modelLists[p]) && (
                        <>
                          {" · "}
                          <button
                            className="link"
                            onClick={() => setManual((m) => ({ ...m, [p]: false }))}
                          >
                            Wybierz z listy
                          </button>
                        </>
                      )}
                      {/* A typed key is sent to the provider only on this click, never
                          on its own; with the stored key it is a retry after a failure. */}
                      {(typedKey(p)
                        ? !validateApiKey(p, typedKey(p))
                        : keysPresent[p] && typeof modelLists[p] === "string") && (
                        <>
                          {" · "}
                          <button
                            className="link"
                            data-ux={`models-${p}`}
                            onClick={() => loadModels(p, typedKey(p) || undefined)}
                          >
                            {typedKey(p) ? "Pobierz listę nowym kluczem" : "Spróbuj ponownie"}
                          </button>
                        </>
                      )}
                    </span>
                  </label>
                  <label className="field">
                    Klucz API
                    <input
                      type="password"
                      placeholder={keysPresent[p] ? "•••••••• bez zmian" : "wklej klucz"}
                      value={apiKeys[p] ?? ""}
                      onChange={(e) => setApiKeys((k) => ({ ...k, [p]: e.target.value }))}
                    />
                    <span className="field__hint">Skąd wziąć: {KEY_HINTS[p]}</span>
                    {errors[`key.${p}`] && (
                      <span className="field__err">{errors[`key.${p}`]}</span>
                    )}
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
                  </label>
                  {p === "openai" && (
                    <label className="field">
                      Długość odpowiedzi (verbosity)
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
                  )}
                </div>
              ))}
            </div>
          </section>
        </div>

        <footer className="modal__actions">
          <button onClick={migrate}>Migruj config.ini</button>
          <span className="modal__status" role="status">
            {status}
          </span>
          <button onClick={close}>Anuluj</button>
          <button className="primary" onClick={save}>
            Zapisz
          </button>
        </footer>
      </div>
    </dialog>
  );
}
