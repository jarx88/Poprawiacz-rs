import { PanelState, Provider, PROVIDER_META } from "./types";

interface Props {
  provider: Provider;
  state: PanelState;
  onUse: (text: string) => void;
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
        ? `✅ Gotowe (${(state.elapsedMs / 1000).toFixed(1)}s)`
        : "✅ Gotowe";
    case "error":
      return "⚠️ Błąd";
  }
}

export function ProviderPanel({ provider, state, onUse }: Props) {
  const meta = PROVIDER_META[provider];
  const canUse = state.status === "done" && state.text.trim() !== "";

  return (
    <section className="panel">
      <header className="panel__head" style={{ background: meta.color }}>
        <span className="panel__title">🤖 {meta.label}</span>
        <span className="panel__status">{statusLabel(state)}</span>
      </header>

      <div className="panel__body">
        {state.status === "error" ? (
          <p className="panel__error">{state.error}</p>
        ) : (
          <pre className="panel__text">
            {state.text || (state.status === "loading" ? "" : " ")}
          </pre>
        )}
      </div>

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
