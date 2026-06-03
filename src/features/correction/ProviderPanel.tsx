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
}

function statusLabel(state: PanelState): string {
  switch (state.status) {
    case "idle":
      return "—";
    case "loading":
      return "⏳ Przetwarzanie…";
    case "streaming":
      return "✍️ Strumieniowanie…";
    case "done":
      return state.elapsedMs != null
        ? `✅ ${(state.elapsedMs / 1000).toFixed(1)}s`
        : "✅ Gotowe";
    case "error":
      return "⚠️ Błąd";
    case "cancelled":
      return "❌ Anulowano";
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
}: Props) {
  const meta = PROVIDER_META[provider];
  const [menuOpen, setMenuOpen] = useState(false);
  const canUse = state.status === "done" && state.text.trim() !== "";
  const busy = state.status === "loading" || state.status === "streaming";

  const body = () => {
    if (state.status === "error") {
      return <p className="panel__error">{state.error}</p>;
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
    <section className="panel">
      <header className="panel__head" style={{ background: meta.color }}>
        <span className="panel__title">🤖 {meta.label}</span>
        <span className="panel__head-right">
          <span className="panel__status">{statusLabel(state)}</span>
          <span className="panel__menu-wrap">
            <button
              className="panel__icon"
              title="Akcje"
              onClick={() => setMenuOpen((o) => !o)}
            >
              ⚙️
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

      <div className="panel__body">{body()}</div>

      <button
        className="panel__use"
        style={{ background: canUse ? meta.color : undefined }}
        disabled={!canUse}
        onClick={() => onUse(state.text)}
      >
        📋 Użyj {meta.label}
      </button>
    </section>
  );
}
