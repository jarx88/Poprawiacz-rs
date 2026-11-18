import { useEffect, useRef, useState } from "react";
import { Icon } from "../../Icon";
import {
  AiSettings,
  getSettings,
  type KeyAccount,
  listModels,
  listOpenrouterModels,
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

const LEVELS = ["off", "low", "medium", "high", "max"] as const;
const LEVEL_LABELS: Record<string, string> = {
  off: "Off (najszybciej)",
  low: "Low",
  medium: "Medium",
  high: "High",
  max: "Max",
};
const VERBOSITIES = ["low", "medium", "high"];

/** Per panel: the fetched model names, or why they could not be fetched. */
type ModelLists = Partial<Record<Provider, string[] | string>>;

/** The model name: a choice from the provider's list once it is fetched, a text
 *  field until then, without a key, or in manual mode. */
function ModelField(props: {
  value: string;
  /** The name the dialog opened with: stays selectable even when the provider
   *  no longer lists it. */
  saved: string;
  list: string[] | string | undefined;
  manual: boolean;
  onChange: (model: string) => void;
}) {
  const { value, saved, list, manual, onChange } = props;
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
    <select value={value} onChange={(e) => onChange(e.target.value)}>
      {[...unlisted, ...list].map((id) => (
        <option key={id} value={id}>
          {id || "— wybierz model —"}
        </option>
      ))}
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
  const [apiKeys, setApiKeys] = useState<Partial<Record<KeyAccount, string>>>({});
  const [errors, setErrors] = useState<Record<string, string | undefined>>({});
  const [status, setStatus] = useState<string>("");
  // per provider: the fetched model names, or why they could not be fetched
  const [modelLists, setModelLists] = useState<ModelLists>({});
  const [manual, setManual] = useState<Partial<Record<Provider, boolean>>>({});
  const [savedModels, setSavedModels] = useState<Record<Provider, string>>(emptyModels());
  // The OpenRouter route: one key for every panel and its own model ids, kept
  // apart from the direct ones because the two routes name a model differently.
  const [openrouter, setOpenrouter] = useState(false);
  const [orModels, setOrModels] = useState<Record<Provider, string>>(emptyModels());
  const [savedOrModels, setSavedOrModels] = useState<Record<Provider, string>>(emptyModels());
  const [orLists, setOrLists] = useState<ModelLists>({});
  const { ref, close } = useModal();

  // Only the newest request of a provider may set its list: an older one
  // (stored key) can come back after a newer one (key just typed).
  const requests = useRef<Partial<Record<Provider, number>>>({});
  const typedKey = (account: KeyAccount) => (apiKeys[account] ?? "").trim();
  /** The typed key was removed again: its list goes at once, then comes what
   *  the stored key lists. */
  const restoreModels = (p: Provider) => {
    requests.current[p] = (requests.current[p] ?? 0) + 1; // drop a pending answer
    setModelLists((l) => ({ ...l, [p]: undefined }));
    if (keysPresent[p]) void loadModels(p);
  };
  const loadModels = (p: Provider, apiKey?: string) => {
    const request = (requests.current[p] = (requests.current[p] ?? 0) + 1);
    return listModels(p, apiKey)
      .then((list) => (list.length ? list : "Dostawca nie zwrócił żadnego modelu."))
      .catch((e) => String(e))
      .then((result) => {
        if (requests.current[p] === request) setModelLists((l) => ({ ...l, [p]: result }));
      });
  };

  /** OpenRouter's list is public and covers every panel in one request, so one
   *  request at a time is enough (and none can overtake another). */
  const orLoading = useRef(false);
  const loadOrModels = () => {
    if (orLoading.current) return;
    orLoading.current = true;
    const all = (value: string) => Object.fromEntries(PROVIDERS.map((p) => [p, value]));
    listOpenrouterModels()
      .then((lists) =>
        PROVIDERS.some((p) => lists[p]?.length)
          ? lists
          : all("OpenRouter nie zwrócił żadnego modelu."),
      )
      .catch((e) => all(String(e)))
      .then((lists) => {
        orLoading.current = false;
        setOrLists(lists);
      });
  };
  /** Lists are fetched for the route in use only: opening the dialog in
   *  OpenRouter mode sends nothing to the providers. */
  const switchRoute = (on: boolean, present = keysPresent) => {
    setOpenrouter(on);
    setManual({}); // manual entry belongs to the route it was chosen on
    if (on) {
      if (!PROVIDERS.some((p) => Array.isArray(orLists[p]))) loadOrModels();
    } else {
      for (const p of PROVIDERS)
        if (present[p] && modelLists[p] === undefined) void loadModels(p);
    }
  };

  const applyView = (s: Awaited<ReturnType<typeof getSettings>>) => {
    const names = (from: Record<Provider, string | null>) =>
      Object.fromEntries(PROVIDERS.map((p) => [p, from[p] ?? ""])) as Record<Provider, string>;
    setModels(names(s.models));
    setSavedModels(names(s.models));
    setOrModels(names(s.openrouter_models));
    setSavedOrModels(names(s.openrouter_models));
    switchRoute(s.openrouter, s.keys_present ?? {});
    setDefaultStyle(s.default_style || "normal");
    setHighlightDiffs(s.highlight_diffs);
    setAutostartup(s.autostartup);
    setClipboardDelayMs(s.clipboard_delay_ms);
    setAi({ ...s.ai_settings, system_prompt: s.ai_settings.system_prompt ?? "" });
    setDefaultPrompt(s.default_system_prompt);
    setKeysPresent(s.keys_present ?? {});
  };

  useEffect(() => {
    getSettings()
      .then(applyView)
      .catch((e) => setStatus(`Nie udało się wczytać ustawień: ${e}`));
  }, []);

  const save = async () => {
    // Only the route in use is validated and saved: the other one's fields are
    // hidden, so a slip typed there must neither block the save nor be stored.
    // An emptied field means "leave the stored key"; sending "" would delete it.
    const accounts: readonly KeyAccount[] = openrouter ? ["openrouter"] : PROVIDERS;
    const keys = Object.fromEntries(
      accounts.map((account) => [account, typedKey(account)]).filter(([, key]) => key),
    );
    const result = validateSettings({
      models: openrouter ? orModels : models,
      defaultStyle,
      apiKeys: keys,
    });
    if (openrouter && !keysPresent.openrouter && !typedKey("openrouter"))
      result.errors["key.openrouter"] = "Połączenie przez OpenRouter wymaga jego klucza.";
    setErrors(result.errors);
    if (Object.keys(result.errors).length) return;
    try {
      await saveSettings({
        models,
        openrouter,
        openrouter_models: orModels,
        default_style: defaultStyle,
        highlight_diffs: highlightDiffs,
        autostartup,
        ai_settings: ai,
        clipboard_delay_ms: clipboardDelayMs,
        api_keys: keys,
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
              <label className="field">
                Połączenie
                <select
                  data-ux="route"
                  value={openrouter ? "openrouter" : "direct"}
                  onChange={(e) => switchRoute(e.target.value === "openrouter")}
                >
                  <option value="direct">Własne klucze dostawców</option>
                  <option value="openrouter">OpenRouter (jeden klucz dla wszystkich)</option>
                </select>
              </label>
              {openrouter && (
                <label className="field">
                  Klucz OpenRouter
                  <input
                    type="password"
                    placeholder={keysPresent.openrouter ? "•••••••• bez zmian" : "wklej klucz"}
                    value={apiKeys.openrouter ?? ""}
                    onChange={(e) => setApiKeys((k) => ({ ...k, openrouter: e.target.value }))}
                  />
                  <span className="field__hint">Skąd wziąć: openrouter.ai/keys</span>
                  {errors["key.openrouter"] && (
                    <span className="field__err">{errors["key.openrouter"]}</span>
                  )}
                </label>
              )}
            </div>
            <div className="group__grid">
              {PROVIDERS.map((p) => {
                // the route in use decides which key, model and list a card shows
                const hasKey = keysPresent[openrouter ? "openrouter" : p];
                const list = openrouter ? orLists[p] : modelLists[p];
                const setModel = openrouter ? setOrModels : setModels;
                return (
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
                    <span className={`chip chip--${hasKey ? "ok" : "warn"}`}>
                      {hasKey ? "klucz zapisany" : "brak klucza"}
                    </span>
                  </div>
                  <label className="field">
                    Model
                    <ModelField
                      // a fresh field per route: nothing typed carries over
                      key={openrouter ? "openrouter" : "direct"}
                      value={(openrouter ? orModels : models)[p]}
                      saved={(openrouter ? savedOrModels : savedModels)[p]}
                      list={list}
                      manual={manual[p] ?? false}
                      onChange={(model) => setModel((m) => ({ ...m, [p]: model }))}
                    />
                    {errors[`model.${p}`] && (
                      <span className="field__err">{errors[`model.${p}`]}</span>
                    )}
                    <span className="field__hint">
                      {typeof list === "string" && <>Lista modeli niedostępna: {list} </>}
                      {!openrouter && !hasKey && list === undefined && (
                        <>Lista modeli wymaga klucza API. </>
                      )}
                      <button
                        className="link"
                        onClick={() =>
                          openUrl(openrouter ? "https://openrouter.ai/models" : MODEL_DOCS[p]).catch(
                            (e) => setStatus(String(e)),
                          )
                        }
                      >
                        {openrouter ? "Modele w OpenRouter" : "Nazwy modeli u dostawcy"}
                      </button>
                      {/* one toggle that stays in place, so it keeps the keyboard focus */}
                      {Array.isArray(list) && (
                        <>
                          {" · "}
                          <button
                            className="link"
                            data-ux={`manual-${p}`}
                            onClick={() => setManual((m) => ({ ...m, [p]: !m[p] }))}
                          >
                            {manual[p] ? "Wybierz z listy" : "Wpisz ręcznie"}
                          </button>
                        </>
                      )}
                      {openrouter && typeof list === "string" && (
                        <>
                          {" · "}
                          <button className="link" onClick={loadOrModels}>
                            Spróbuj ponownie
                          </button>
                        </>
                      )}
                      {/* A typed key is sent to the provider only on this click, never
                          on its own; with the stored key it is a retry after a failure. */}
                      {!openrouter &&
                        (typedKey(p)
                          ? !validateApiKey(p, typedKey(p))
                          : hasKey && typeof list === "string") && (
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
                  {!openrouter && (
                  <label className="field">
                    Klucz API
                    <input
                      type="password"
                      placeholder={keysPresent[p] ? "•••••••• bez zmian" : "wklej klucz"}
                      value={apiKeys[p] ?? ""}
                      onChange={(e) => {
                        setApiKeys((k) => ({ ...k, [p]: e.target.value }));
                        if (typedKey(p) && !e.target.value.trim()) restoreModels(p);
                      }}
                    />
                    <span className="field__hint">Skąd wziąć: {KEY_HINTS[p]}</span>
                    {errors[`key.${p}`] && (
                      <span className="field__err">{errors[`key.${p}`]}</span>
                    )}
                  </label>
                  )}
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
                  {/* verbosity belongs to OpenAI's own Responses API */}
                  {p === "openai" && !openrouter && (
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
                );
              })}
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
