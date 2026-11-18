import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { Icon } from "../../Icon";
import {
  cancelProvider,
  cancelSession,
  getSettings,
  onCancelled,
  onChunk,
  onError,
  onHotkeyEmpty,
  onResult,
  onRestarted,
  onSessionStarted,
  pasteText,
  readClipboard,
  rememberCorrection,
  reprocessProvider,
  startCorrection,
  writeClipboard,
} from "../../lib/tauri";
import { keyAction } from "./actions";
import { OriginalTextModal } from "./OriginalTextModal";
import { ProviderPanel } from "./ProviderPanel";
import { useCorrectionStore } from "./store";
import { PROVIDERS, Provider, STYLES, STYLE_LABELS, Style } from "./types";

export function CorrectionView() {
  const {
    sessionId,
    originalText,
    panels,
    startSession,
    applyChunk,
    applyResult,
    applyError,
    applyCancelled,
    applyRestarted,
  } = useCorrectionStore();
  const [input, setInput] = useState("");
  const [style, setStyle] = useState<Style>("normal");
  const [highlightDiffs, setHighlightDiffs] = useState(false);
  const [showOriginal, setShowOriginal] = useState(false);
  const [notice, setNotice] = useState<{ text: string; ok?: boolean } | null>(null);

  // A confirmation clears itself; a failure stays until the next action, so it is
  // still there when the window comes back from the tray.
  useEffect(() => {
    if (!notice?.ok) return;
    const t = setTimeout(() => setNotice(null), 6000);
    return () => clearTimeout(t);
  }, [notice]);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setHighlightDiffs(s.highlight_diffs);
        if (s.default_style) setStyle(s.default_style as Style);
      })
      .catch(() => {});
  }, []);

  // Wire backend events to the store once.
  useEffect(() => {
    const unsubs = [
      onSessionStarted((e) => {
        setNotice(null);
        setInput(e.text);
        startSession(e.session_id, e.text);
      }),
      onChunk((e) => applyChunk(e)),
      onResult((e) => applyResult(e)),
      onError((e) => applyError(e)),
      onCancelled((e) => applyCancelled(e)),
      onRestarted((e) => applyRestarted(e)),
      onHotkeyEmpty(() =>
        setNotice({ text: "Schowek jest pusty. Zaznacz tekst i spróbuj ponownie." }),
      ),
    ];
    return () => {
      unsubs.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [applyChunk, applyError, applyResult, applyCancelled, applyRestarted, startSession]);

  const runManual = async () => {
    setNotice(null);
    let text = input;
    if (text.trim() === "") {
      try {
        text = await readClipboard();
      } catch {
        text = "";
      }
    }
    if (text.trim() === "") {
      setNotice({ text: "Brak tekstu do poprawy. Wpisz go lub skopiuj do schowka." });
      return;
    }
    // the backend's "session-started" event resets the panels; doing it here as
    // well could put a panel that already failed back to loading
    await startCorrection(text, style);
  };

  // One paste at a time, for the button and the shortcut alike: a second call
  // would overwrite the clipboard before the first Ctrl+V lands.
  const pasting = useRef(false);
  const onUse = async (text: string) => {
    if (pasting.current) return;
    pasting.current = true;
    setNotice(null);
    try {
      await pasteText(text);
    } catch (e) {
      setNotice({ text: `Wklejanie nie powiodło się: ${e}` });
    } finally {
      pasting.current = false;
    }
  };

  const onCopy = async (text: string) => {
    try {
      await writeClipboard(text);
      setNotice({ text: "Skopiowano do schowka.", ok: true });
    } catch (e) {
      setNotice({ text: `Kopiowanie nie powiodło się: ${e}` });
    }
  };

  // Keys 1–4 use that panel's result, Ctrl+Enter runs the correction. Not while
  // a dialog or a panel menu is open: they own the keyboard then.
  const onKey = useEffectEvent((e: KeyboardEvent) => {
    if (document.querySelector(".modal, .panel__menu")) return;
    const inField = (e.target as Element).closest?.("input, textarea, select") != null;
    const action = keyAction(e, inField);
    if (action === null) return;
    if (action === "run") {
      e.preventDefault();
      void runManual();
      return;
    }
    const panel = panels[PROVIDERS[action]];
    if (panel.status === "done" && panel.text.trim() !== "") {
      e.preventDefault();
      void onUse(panel.text);
    }
  });
  useEffect(() => {
    const listener = (e: KeyboardEvent) => onKey(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  const onLike = async (corrected: string, correctionStyle: Style): Promise<boolean> => {
    try {
      const count = await rememberCorrection(originalText, corrected, correctionStyle);
      setNotice({ text: `Zapamiętano dobry przykład (${count}/5).`, ok: true });
      return true;
    } catch (e) {
      setNotice({ text: `Nie udało się zapamiętać korekty: ${e}` });
      return false;
    }
  };

  // Optimistic cancel: flip the panel(s) immediately, then tell the backend to
  // abort the request (don't wait for the round-trip event).
  const cancelOne = (prov: Provider) => {
    applyCancelled({ session_id: sessionId, provider: prov });
    cancelProvider(prov);
  };
  const cancelAll = () => {
    PROVIDERS.forEach((p) => applyCancelled({ session_id: sessionId, provider: p }));
    cancelSession();
  };

  const reprocess = (prov: Provider, st: string) => {
    setNotice(null);
    reprocessProvider(prov, st);
  };

  const anyBusy = PROVIDERS.some((p) =>
    ["loading", "streaming"].includes(panels[p].status),
  );
  const noSession = PROVIDERS.every((p) => panels[p].status === "idle");

  return (
    <div className="correction">
      <nav className="tabs" aria-label="Styl korekty">
        {STYLES.map((s) => (
          <button
            key={s}
            className={`tab${s === style ? " tab--active" : ""}`}
            data-label={STYLE_LABELS[s]}
            aria-current={s === style ? "true" : undefined}
            onClick={() => setStyle(s)}
          >
            {STYLE_LABELS[s]}
          </button>
        ))}
      </nav>

      {/* The text and everything that acts on it live in one card. */}
      <div className="composer">
        <textarea
          className="composer__input"
          aria-label="Tekst do poprawy"
          placeholder="Wklej lub wpisz tekst albo zaznacz go gdziekolwiek i naciśnij Ctrl+Shift+C"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <div className="composer__bar">
          <button
            className="ghost"
            data-ux="original"
            title="Pokaż tekst sprzed korekty"
            onClick={() => setShowOriginal(true)}
            disabled={!originalText}
          >
            <Icon name="file" />
            Oryginał
          </button>
          <button
            className="ghost"
            title="Ukryj okno do zasobnika"
            onClick={() => getCurrentWindow().hide()}
          >
            <Icon name="tray" />
            Minimalizuj
          </button>
          <span className="composer__notice" role="status">
            {notice && (
              <span
                className={`chip chip--${notice.ok ? "ok" : "warn"}`}
                title={notice.text}
              >
                <Icon name={notice.ok ? "check" : "alert"} size={14} />
                <span>{notice.text}</span>
              </span>
            )}
          </span>
          <button
            className="ghost ghost--danger"
            title="Przerwij wszystkie trwające zapytania"
            onClick={cancelAll}
            disabled={!anyBusy}
          >
            <Icon name="stop" />
            Anuluj wszystko
          </button>
          {/* Once a session runs or an answer is ready, "Użyj" is the primary action
              and this one steps back. */}
          <button
            className={`cta${noSession ? "" : " cta--quiet"}`}
            title="Popraw tekst (Ctrl+Enter)"
            onClick={runManual}
          >
            Popraw
          </button>
        </div>
      </div>

      <div className={`grid${noSession ? " grid--empty" : ""}`}>
        {PROVIDERS.map((p, i) => (
          <ProviderPanel
            key={p}
            provider={p}
            shortcut={i + 1}
            state={panels[p]}
            originalText={originalText}
            highlightDiffs={highlightDiffs}
            onUse={onUse}
            onCopy={onCopy}
            onLike={onLike}
            onCancel={(prov: Provider) => cancelOne(prov)}
            onReprocess={reprocess}
            onRetry={(prov: Provider) => reprocess(prov, style)}
          />
        ))}
      </div>
      {noSession && (
        <div className="empty">
          <strong>
            Zaznacz tekst w dowolnym programie i naciśnij <kbd>Ctrl+Shift+C</kbd>
          </strong>
          <span>
            Cztery poprawione wersje pojawią się tutaj obok siebie. Możesz też wkleić tekst
            powyżej i kliknąć Popraw.
          </span>
        </div>
      )}

      {showOriginal && (
        <OriginalTextModal text={originalText} onClose={() => setShowOriginal(false)} />
      )}
    </div>
  );
}
