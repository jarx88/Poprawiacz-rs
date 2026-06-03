import { useEffect, useState } from "react";
import {
  cancelSession,
  onChunk,
  onError,
  onResult,
  onSessionStarted,
  pasteText,
  readClipboard,
  startCorrection,
} from "../../lib/tauri";
import { ProviderPanel } from "./ProviderPanel";
import { useCorrectionStore } from "./store";
import { PROVIDERS, STYLES, Style } from "./types";

export function CorrectionView() {
  const { sessionId, panels, startSession, applyChunk, applyResult, applyError } =
    useCorrectionStore();
  const [input, setInput] = useState("");
  const [style, setStyle] = useState<Style>("normal");

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
    ];
    return () => {
      unsubs.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [applyChunk, applyError, applyResult, startSession]);

  const runManual = async () => {
    let text = input;
    if (text.trim() === "") {
      try {
        text = await readClipboard();
      } catch {
        text = "";
      }
    }
    if (text.trim() === "") return;
    const id = await startCorrection(text, style);
    startSession(id, text);
  };

  const onUse = async (text: string) => {
    try {
      await pasteText(text);
    } catch (e) {
      console.error("paste failed", e);
    }
  };

  return (
    <div className="correction">
      <div className="correction__bar">
        <span className="correction__session">📝 Sesja: {sessionId}</span>
        <select
          value={style}
          onChange={(e) => setStyle(e.target.value as Style)}
          aria-label="Styl korekty"
        >
          {STYLES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <button onClick={runManual}>▶️ Popraw</button>
        <button onClick={() => cancelSession()}>❌ Anuluj wszystko</button>
      </div>

      <textarea
        className="correction__input"
        placeholder="Wklej lub wpisz tekst, albo użyj Ctrl+Shift+C…"
        value={input}
        onChange={(e) => setInput(e.target.value)}
      />

      <div className="grid">
        {PROVIDERS.map((p) => (
          <ProviderPanel key={p} provider={p} state={panels[p]} onUse={onUse} />
        ))}
      </div>
    </div>
  );
}
