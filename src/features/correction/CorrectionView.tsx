import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
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
  reprocessProvider,
  startCorrection,
} from "../../lib/tauri";
import { OriginalTextModal } from "./OriginalTextModal";
import { ProviderPanel } from "./ProviderPanel";
import { completedCount, useCorrectionStore } from "./store";
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
  const [notice, setNotice] = useState<string | null>(null);

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
        setInput(e.text);
        startSession(e.session_id, e.text);
      }),
      onChunk((e) => applyChunk(e)),
      onResult((e) => applyResult(e)),
      onError((e) => applyError(e)),
      onCancelled((e) => applyCancelled(e)),
      onRestarted((e) => applyRestarted(e)),
      onHotkeyEmpty(() => setNotice("Schowek jest pusty — zaznacz tekst i spróbuj ponownie.")),
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
      setNotice("Brak tekstu do poprawy.");
      return;
    }
    const id = await startCorrection(text, style);
    startSession(id, text);
  };

  const onUse = async (text: string) => {
    try {
      await pasteText(text);
    } catch (e) {
      setNotice(`Wklejanie nie powiodło się: ${e}`);
    }
  };

  const done = completedCount(panels);

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

  return (
    <div className="correction">
      <div className="correction__bar">
        <span className="correction__session">📝 Sesja: {sessionId}</span>
        <span className="correction__counter">🤖 API: {done}/4</span>
        <select
          value={style}
          onChange={(e) => setStyle(e.target.value as Style)}
          aria-label="Styl korekty"
        >
          {STYLES.map((s) => (
            <option key={s} value={s}>
              {STYLE_LABELS[s]}
            </option>
          ))}
        </select>
        <button onClick={runManual}>▶️ Popraw</button>
        <button onClick={cancelAll}>❌ Anuluj wszystko</button>
        <button onClick={() => setShowOriginal(true)} disabled={!originalText}>
          📄 Oryginał
        </button>
        <button onClick={() => getCurrentWindow().hide()}>🔽 Minimalizuj</button>
      </div>

      {notice && <div className="notice">{notice}</div>}

      <textarea
        className="correction__input"
        placeholder="Wklej lub wpisz tekst, albo użyj Ctrl+Shift+C…"
        value={input}
        onChange={(e) => setInput(e.target.value)}
      />

      <div className="grid">
        {PROVIDERS.map((p) => (
          <ProviderPanel
            key={p}
            provider={p}
            state={panels[p]}
            originalText={originalText}
            highlightDiffs={highlightDiffs}
            onUse={onUse}
            onCancel={(prov: Provider) => cancelOne(prov)}
            onReprocess={(prov: Provider, st: string) => reprocessProvider(prov, st)}
          />
        ))}
      </div>

      {showOriginal && (
        <OriginalTextModal text={originalText} onClose={() => setShowOriginal(false)} />
      )}
    </div>
  );
}
