import { useState } from "react";
import { diffWords } from "./diff";
import { PanelState, Provider, PROVIDER_META } from "./types";

interface Props {
  provider: Provider;
  state: PanelState;
  originalText: string;
  highlightDiffs: boolean;
  onUse: (text: string) => void;
  onCancel: (provider: Provider) => void;
  onReprocess: (provider: Provider, style: string) => void;
  onRetry: (provider: Provider) => void;
}

function statusLabel(state: PanelState): string {
  switch (state.status) {
    case "idle":
      return "—";
    case "loading":
      return "przetwarzanie…";
    case "streaming":
      return "strumieniowanie…";
    case "done":
      return state.elapsedMs != null
        ? `${(state.elapsedMs / 1000).toFixed(1)}s`
        : "gotowe";
    case "error":
      return "błąd";
    case "cancelled":
      return "anulowano";
  }
}

const REPROCESS_ACTIONS: { label: string; style: string }[] = [
  { label: "Profesjonalny", style: "professional" },
  { label: "Tłumacz EN", style: "translate_en" },
  { label: "Tłumacz PL", style: "translate_pl" },
  { label: "Podsumowanie", style: "summary" },
];

export function ProviderPanel({
  provider,
  state,
  originalText,
  highlightDiffs,
  onUse,
  onCancel,
  onReprocess,
  onRetry,
}: Props) {
  const meta = PROVIDER_META[provider];
  const [menuOpen, setMenuOpen] = useState(false);
  const canUse = state.status === "done" && state.text.trim() !== "";
  const busy = state.status === "loading" || state.status === "streaming";
  const failed = state.status === "error" || state.status === "cancelled";

  const body = () => {
    if (state.status === "error") {
      return <p className="panel__error">{state.error}</p>;
    }
    if (state.status === "cancelled") {
      return <p className="panel__error">Anulowano.</p>;
    }
    if (busy && state.text === "") {
      return <div className="panel__spinner" aria-label="Ładowanie" />;
    }
    if (highlightDiffs && state.status === "done" && originalText) {
      return (
        <pre className="panel__text">
          {diffWords(originalText, state.text).map((seg, i) =>
            seg.changed ? (
              <mark key={i} className="diff">
                {seg.text}
              </mark>
            ) : (
              <span key={i}>{seg.text}</span>
            ),
          )}
        </pre>
      );
    }
    return <pre className="panel__text">{state.text || " "}</pre>;
  };

  return (
    <section
      className={`panel${failed ? " panel--failed" : ""}`}
      style={{ "--pc": meta.color } as React.CSSProperties}
    >
      <header className="panel__head">
        <span className="panel__title">
          {meta.label} <span className="panel__dot" />
        </span>
        <span className="panel__head-right">
          <span className="panel__status">{statusLabel(state)}</span>
          <span className="panel__menu-wrap">
            <button
              className="panel__icon"
              title="Akcje"
              onClick={() => setMenuOpen((o) => !o)}
            >
              ⚙
            </button>
            {menuOpen && (
              <div className="panel__menu" onMouseLeave={() => setMenuOpen(false)}>
                {REPROCESS_ACTIONS.map((a) => (
                  <button
                    key={a.style}
                    onClick={() => {
                      setMenuOpen(false);
                      onReprocess(provider, a.style);
                    }}
                  >
                    {a.label}
                  </button>
                ))}
              </div>
            )}
          </span>
          {busy && (
            <button
              className="panel__icon"
              title="Anuluj"
              onClick={() => onCancel(provider)}
            >
              ✖
            </button>
          )}
        </span>
      </header>

      {busy && (
        <div
          className="panel__progress"
          role="progressbar"
          aria-label="Trwa przetwarzanie"
        >
          <div className="panel__progress-bar" />
        </div>
      )}

      <div className="panel__body">{body()}</div>

      {failed ? (
        <button className="panel__use panel__use--retry" onClick={() => onRetry(provider)}>
          Ponów ↻
        </button>
      ) : (
        <button
          className="panel__use"
          disabled={!canUse}
          onClick={() => onUse(state.text)}
        >
          Użyj {meta.label}
        </button>
      )}
    </section>
  );
}
