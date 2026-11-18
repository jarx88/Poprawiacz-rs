// Capture harness: the real App with Tauri IPC mocked and the store put into one
// fixed state, chosen by ?state=. capture.mjs awaits __uxReady, then reads __uxMetrics().
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import ReactDOM from "react-dom/client";
import App from "../src/App";
import { useCorrectionStore } from "../src/features/correction/store";
import type { PanelState, Provider } from "../src/features/correction/types";
import "../src/styles.css";

const ORIGINAL =
  "Cześć Marku, przesyłam ci podsumowanie wczorajszego spotkania wstępnie ustaliliśmy że wdrożenie ruszy w przyszły poniedziałek ale musimy jeszcze potwierdzic dostępność zespołu. Daj znać czy pasuje ci czwartek na krótką rozmowe.";

const FIXED: Record<Provider, string> = {
  openai:
    "Cześć Marku, przesyłam Ci podsumowanie wczorajszego spotkania. Wstępnie ustaliliśmy, że wdrożenie ruszy w przyszły poniedziałek, ale musimy jeszcze potwierdzić dostępność zespołu. Daj znać, czy pasuje Ci czwartek na krótką rozmowę.",
  anthropic:
    "Cześć Marku, przesyłam ci podsumowanie wczorajszego spotkania. Wstępnie ustaliliśmy, że wdrożenie ruszy w przyszły poniedziałek, ale musimy jeszcze potwierdzić dostępność zespołu. Daj znać, czy pasuje ci czwartek na krótką rozmowę.",
  gemini:
    "Cześć Marku, przesyłam Ci podsumowanie wczorajszego spotkania. Wstępnie ustaliliśmy, że wdrożenie rozpocznie się w przyszły poniedziałek, ale musimy jeszcze potwierdzić dostępność zespołu. Daj znać, czy odpowiada Ci czwartek na krótką rozmowę.",
  deepseek:
    "Cześć Marku, przesyłam ci podsumowanie wczorajszego spotkania. Wstępnie ustaliliśmy, że wdrożenie ruszy w przyszły poniedziałek, ale musimy jeszcze potwierdzić dostępność zespołu. Daj znać, czy pasuje ci czwartek na krótką rozmowę.",
};

const LONG = [1, 2, 3, 4]
  .map(
    (n) =>
      `Punkt ${n}. Po analizie zgłoszeń z ostatniego kwartału widać, że większość problemów dotyczy synchronizacji danych między oddziałami. Proponujemy wdrożyć poprawkę etapami, zaczynając od najmniejszego oddziału, i po tygodniu ocenić wyniki.`,
  )
  .join("\n\n");

const SETTINGS = {
  models: {
    openai: "gpt-5-mini",
    anthropic: "claude-3-7-sonnet-latest",
    gemini: "gemini-3.6-flash",
    deepseek: "deepseek-chat",
  },
  default_style: "normal",
  highlight_diffs: true,
  autostartup: true,
  ai_settings: {
    verbosity: "medium",
    reasoning_levels: { openai: "off", anthropic: "off", gemini: "low", deepseek: "off" },
    system_prompt: "",
  },
  clipboard_delay_ms: 400,
  keys_present: { openai: true, anthropic: true, gemini: true, deepseek: false },
  default_system_prompt: "Jesteś redaktorem. Popraw tekst, zachowując styl autora.",
};

const panel = (p: Partial<PanelState>): PanelState => ({
  status: "idle",
  text: "",
  error: null,
  elapsedMs: null,
  style: "normal",
  attempt: 0,
  ...p,
});
const done = (prov: Provider, ms: number, text = FIXED[prov]) =>
  panel({ status: "done", text, elapsedMs: ms });
const allDone = (text?: string): Record<Provider, PanelState> => ({
  openai: done("openai", 4210, text),
  anthropic: done("anthropic", 1630, text),
  gemini: done("gemini", 1140, text),
  deepseek: done("deepseek", 2870, text),
});

const STATES: Record<string, Record<Provider, PanelState> | null> = {
  idle: null,
  loading: {
    openai: panel({ status: "loading" }),
    anthropic: panel({ status: "loading" }),
    gemini: panel({ status: "loading" }),
    deepseek: panel({ status: "loading" }),
  },
  mixed: {
    openai: panel({ status: "streaming", text: FIXED.openai.slice(0, 96) }),
    anthropic: done("anthropic", 1630),
    gemini: done("gemini", 1140),
    deepseek: panel({ status: "loading" }),
  },
  done: allDone(),
  errors: {
    openai: done("openai", 4210),
    anthropic: panel({
      status: "error",
      // the backend's real wording (AiError::user_message)
      error:
        "Dostawca odrzucił klucz API. Sprawdź klucz w ustawieniach. Szczegóły (HTTP 401): invalid x-api-key",
    }),
    gemini: panel({ status: "cancelled" }),
    deepseek: done("deepseek", 2870),
  },
  menu: allDone(),
  long: allDone(LONG),
  settings: allDone(),
  original: allDone(),
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const click = (sel: string) => document.querySelector<HTMLElement>(sel)?.click();

async function main() {
  const q = new URLSearchParams(location.search);
  // "<state>-end" is the same state scrolled to the bottom (checks the scroll fade);
  // "<state>-keys" presses the shortcuts and fails the capture if they do nothing
  const [state, variant] = (q.get("state") ?? "done").split("-");
  const panels = STATES[state];
  const calls: Record<string, unknown> = {};

  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      calls[cmd] = payload;
      if (cmd === "app_build")
        return { version: "0.1.0", build: "113b018", built_at: 1793044080 };
      if (cmd === "get_settings") return SETTINGS;
      if (cmd === "list_models") {
        const { provider } = payload as { provider: Provider };
        // Gemini's list fails: the field stays a text input and says why
        if (provider === "gemini")
          throw "Dostawca odrzucił klucz API. Sprawdź klucz w ustawieniach. (HTTP 401)";
        return [SETTINGS.models[provider], `${provider}-nowszy-model`];
      }
      if (cmd === "remember_correction") return 2;
      return undefined;
    },
    { shouldMockEvents: true },
  );

  ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
  await sleep(100); // listeners register asynchronously

  if (panels) {
    const text = state === "long" ? LONG : ORIGINAL;
    await emit("session-started", { session_id: 7, text });
    useCorrectionStore.setState({ sessionId: 7, originalText: text, panels });
    await sleep(50);
  }
  // Interactions go through data-ux hooks so restyling does not break the harness.
  if (state === "errors") click('[data-ux="like-openai"]');
  if (state === "menu")
    click(`[data-ux="menu-${variant === "low" ? "deepseek" : "anthropic"}"]`);
  if (state === "original") {
    // focused first, as a real click would: closing must hand the focus back
    document.querySelector<HTMLElement>('[data-ux="original"]')!.focus();
    click('[data-ux="original"]');
  }
  if (state === "settings") await emit("open-settings");
  // The settings dialog is a lazy chunk that then loads its data: wait for it.
  const want = { settings: ".provider-card", original: ".original__text", menu: ".panel__menu" }[state];
  for (let i = 0; want && !document.querySelector(want) && i < 100; i++) await sleep(50);
  await sleep(150);
  if (variant === "end")
    for (const el of document.querySelectorAll(".modal__body, .panel__body"))
      el.scrollTop = el.scrollHeight;
  if (variant === "keys") {
    const press = (init: KeyboardEventInit) =>
      window.dispatchEvent(new KeyboardEvent("keydown", init));
    const chip = () => document.querySelector(".composer__notice .chip");
    if (state === "done") await emit("hotkey-empty"); // a failure notice to clear
    press({ key: "2" }); // second panel: Anthropic
    await sleep(50);
    if (chip()) throw new Error("a successful paste left the old failure notice");
    press({ key: "Enter", ctrlKey: true });
    await sleep(100);
    const pasted = (calls.paste_text as { text?: string } | undefined)?.text;
    const ran = "start_correction" in calls;
    // with a panel menu open the keys must do nothing; otherwise both must fire
    const ok = state === "menu" ? !pasted && !ran : pasted === FIXED.anthropic && ran;
    if (!ok) throw new Error(`shortcuts misbehaved in "${state}": ${Object.keys(calls)}`);
  }
  if (variant === "notice") {
    // a failure notice outlives the 6 s of a confirmation, until the next session
    const chip = () => document.querySelector(".composer__notice .chip");
    await emit("hotkey-empty");
    await sleep(6300);
    if (!chip()) throw new Error("the failure notice cleared itself");
    await emit("session-started", { session_id: 8, text: ORIGINAL });
    await sleep(50);
    if (chip()) throw new Error("the failure notice survived a new session");
  }
  if (variant === "blur") {
    // focus leaving the menu button and its list closes the menu
    if (!document.querySelector(".panel__menu")) throw new Error("menu never opened");
    document.querySelector<HTMLElement>('[data-ux="menu-anthropic"]')!.focus();
    document.querySelector<HTMLElement>(".composer__input")!.focus();
    await sleep(50);
    if (document.querySelector(".panel__menu")) throw new Error("menu stayed open after blur");
  }
  if (variant === "focus") {
    // a modal dialog: focus starts on its text, cannot leave it, returns to the opener
    const active = () => document.activeElement;
    if (!active()?.matches(".original__text")) throw new Error("the dialog did not focus its text");
    document.querySelector<HTMLElement>(".composer__input")!.focus();
    if (!active()?.closest("dialog")) throw new Error("the page behind the dialog is not inert");
    click("dialog .primary");
    await sleep(100);
    if (document.querySelector("dialog")) throw new Error("the dialog did not close");
    if (!active()?.matches('[data-ux="original"]')) throw new Error("focus did not return to the opener");
  }
  if (variant === "models") {
    // the fetched list is a select; one toggle switches to a text field and back
    const card = document.querySelector(".provider-card")!; // OpenAI
    const names = [...card.querySelector("select")!.options].map((o) => o.textContent);
    if (!names.includes("openai-nowszy-model")) throw new Error(`no fetched models: ${names}`);
    const toggle = card.querySelector<HTMLElement>('[data-ux="manual-openai"]')!;
    toggle.focus();
    toggle.click();
    await sleep(50);
    if (!document.activeElement?.matches('.provider-card label input:not([type="password"])'))
      throw new Error('"Wpisz ręcznie" did not focus a text field');
    // DeepSeek has no stored key: a typed one lists the models, but only on a click
    const deepseek = [...document.querySelectorAll(".provider-card")].at(-1)!;
    const keyField = deepseek.querySelector<HTMLInputElement>('input[type="password"]')!;
    const type = (text: string) => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(keyField, text);
      keyField.dispatchEvent(new Event("input", { bubbles: true }));
    };
    keyField.focus();
    type("sk-0123456789abcdef");
    keyField.blur();
    await sleep(50);
    if ((calls.list_models as { provider: string }).provider === "deepseek")
      throw new Error("a typed key was sent without a click");
    click('[data-ux="models-deepseek"]');
    await sleep(50);
    const sent = calls.list_models as { provider: string; apiKey?: string };
    if (sent.provider !== "deepseek" || sent.apiKey !== "sk-0123456789abcdef")
      throw new Error(`the typed key did not list the models: ${JSON.stringify(sent)}`);
    if (!deepseek.querySelector("select option[value='deepseek-nowszy-model']"))
      throw new Error("the list fetched with the typed key is not shown");
    // the trial key is removed again: its list must not stay
    type("");
    await sleep(50);
    if (deepseek.querySelector("select option[value='deepseek-nowszy-model']"))
      throw new Error("the list of a removed key is still shown");
  }
  await document.fonts.ready;
}

// --- programmatic metrics: WCAG contrast of visible text, clipped content ------

const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
function rgba(css: string): [number, number, number, number] {
  ctx.clearRect(0, 0, 1, 1);
  ctx.fillStyle = css;
  ctx.fillRect(0, 0, 1, 1);
  const d = ctx.getImageData(0, 0, 1, 1).data;
  return [d[0], d[1], d[2], d[3] / 255];
}
const over = (fg: number[], bg: number[]) =>
  [0, 1, 2].map((i) => fg[i] * fg[3] + bg[i] * (1 - fg[3]));
const lum = (c: number[]) => {
  const [r, g, b] = c.map((v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

function metrics() {
  const lowContrast: object[] = [];
  const clipped: string[] = [];
  for (const el of document.querySelectorAll<HTMLElement>("body *")) {
    const cs = getComputedStyle(el);
    if (!el.getClientRects().length || cs.visibility === "hidden") continue;
    if (
      (cs.overflowX === "hidden" || cs.textOverflow === "ellipsis") &&
      el.scrollWidth > el.clientWidth + 1
    )
      clipped.push(el.className || el.tagName);
    const text = [...el.childNodes]
      .filter((n) => n.nodeType === 3)
      .map((n) => n.textContent!.trim())
      .join(" ")
      .trim();
    if (!text || el.closest(":disabled")) continue;
    // background: composite every ancestor's colour, bottom up
    const chain: HTMLElement[] = [];
    for (let a: HTMLElement | null = el; a; a = a.parentElement) chain.unshift(a);
    let bg = [12, 12, 14];
    let opacity = 1;
    for (const a of chain) {
      const s = getComputedStyle(a);
      bg = over(rgba(s.backgroundColor), bg);
      opacity *= Number(s.opacity);
    }
    const fgc = rgba(cs.color);
    const fg = over([fgc[0], fgc[1], fgc[2], fgc[3] * opacity], bg);
    const [hi, lo] = [lum(fg), lum(bg)].sort((a, b) => b - a);
    const ratio = (hi + 0.05) / (lo + 0.05);
    const px = parseFloat(cs.fontSize);
    const large = px >= 24 || (px >= 18.66 && Number(cs.fontWeight) >= 700);
    if (ratio < (large ? 3 : 4.5))
      lowContrast.push({ el: el.className || el.tagName, text: text.slice(0, 30), ratio: +ratio.toFixed(2), px });
  }
  const root = document.documentElement;
  return {
    lowContrast,
    clipped: [...new Set(clipped)],
    pageOverflow: root.scrollWidth > innerWidth || root.scrollHeight > innerHeight,
  };
}

Object.assign(window, { __uxReady: main(), __uxMetrics: metrics });
