import { useState } from "react";
import { Icon } from "../../Icon";
import { useEscape } from "../../lib/useEscape";
import { diffWords } from "./diff";
import { PanelState, Provider, PROVIDER_META, STYLE_LABELS, Style } from "./types";

interface Props {
  provider: Provider;
  state: PanelState;
  originalText: string;
  highlightDiffs: boolean;
  onUse: (text: string) => void;
  onLike: (text: string, style: Style) => Promise<boolean>;
  onCancel: (provider: Provider) => void;
  onReprocess: (provider: Provider, style: string) => void;
  onRetry: (provider: Provider) => void;
}

function statusLabel(state: PanelState): string {
  switch (state.status) {
    case "idle":
      return "";
    case "loading":
      return "czeka…";
    case "streaming":
      return "pisze…";
    case "done":
      return state.elapsedMs != null
        ? `${(state.elapsedMs / 1000).toFixed(1)} s`
        : "gotowe";
    case "error":
      return "błąd";
    case "cancelled":
      return "anulowano";
  }
}

const REPROCESS_STYLES: Style[] = ["professional", "translate_en", "translate_pl", "summary"];

export function ProviderPanel({
  provider,
  state,
  originalText,
  highlightDiffs,
  onUse,
  onLike,
  onCancel,
  onReprocess,
  onRetry,
}: Props) {
  const meta = PROVIDER_META[provider];
  const [menuOpen, setMenuOpen] = useState(false);
  const [likedKey, setLikedKey] = useState("");
  const [liking, setLiking] = useState(false);
  useEscape(() => setMenuOpen(false));
  const canUse = state.status === "done" && state.text.trim() !== "";
  const likeKey = `${originalText}\u0000${state.style}\u0000${state.text}`;
  const liked = likedKey === likeKey;
  const busy = state.status === "loading" || state.status === "streaming";
  const failed = state.status === "error" || state.status === "cancelled";

  const body = () => {
    if (state.status === "idle") {
      return <p className="panel__note">Tu pojawi się wersja {meta.label}</p>;
    }
    if (state.status === "error") {
      return (
        <p className="panel__error" role="alert">
          {state.error}
        </p>
      );
    }
    if (state.status === "cancelled") {
      return <p className="panel__note">Anulowano. Możesz ponowić tylko ten panel.</p>;
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
      className={`panel panel--${state.status}`}
      style={{ "--pc": meta.color } as React.CSSProperties}
    >
      {busy && (
        <div
          className="panel__progress"
          role="progressbar"
          aria-label="Trwa przetwarzanie"
        >
          <div className="panel__progress-bar" />
        </div>
      )}

      <header className="panel__head">
        <span className="panel__title">
          <span className="panel__dot" />
          {meta.label}
        </span>
        <span className="panel__head-right">
          <span className={`panel__status panel__status--${state.status}`}>
            {state.status === "error" && <Icon name="alert" size={14} />}
            {statusLabel(state)}
          </span>
          {state.status === "done" && (
            <button
              data-ux={`like-${provider}`}
              className={`icon-btn${liked ? " icon-btn--on" : ""}`}
              title={liked ? "Zapamiętano tę korektę" : "Zapamiętaj jako dobry przykład"}
              aria-label={liked ? "Korekta zapamiętana" : "Zapamiętaj korektę"}
              aria-pressed={liked}
              disabled={!canUse || liking || liked}
              onClick={async () => {
                setLiking(true);
                if (await onLike(state.text, state.style)) setLikedKey(likeKey);
                setLiking(false);
              }}
            >
              <Icon name="like" />
            </button>
          )}
          <span className="panel__menu-wrap">
            <button
              data-ux={`menu-${provider}`}
              className="icon-btn"
              title="Przetwórz ponownie w innym stylu"
              aria-label="Przetwórz ponownie w innym stylu"
              aria-haspopup="menu"
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((o) => !o)}
            >
              <Icon name="more" />
            </button>
            {menuOpen && (
              <div
                className="panel__menu"
                role="menu"
                onMouseLeave={() => setMenuOpen(false)}
              >
                <span className="panel__menu-label">Przetwórz ponownie jako</span>
                {REPROCESS_STYLES.map((s) => (
                  <button
                    key={s}
                    role="menuitem"
                    onClick={() => {
                      setMenuOpen(false);
                      onReprocess(provider, s);
                    }}
                  >
                    {STYLE_LABELS[s]}
                  </button>
                ))}
              </div>
            )}
          </span>
          {busy && (
            <button
              className="icon-btn"
              title="Anuluj ten panel"
              aria-label={`Anuluj ${meta.label}`}
              onClick={() => onCancel(provider)}
            >
              <Icon name="close" />
            </button>
          )}
        </span>
      </header>

      <div className="panel__body">{body()}</div>

      {failed ? (
        <button className="panel__use panel__use--retry" onClick={() => onRetry(provider)}>
          <Icon name="retry" /> Ponów {meta.label}
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
