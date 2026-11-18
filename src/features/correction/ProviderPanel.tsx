import { useState } from "react";
import { Icon } from "../../Icon";
import { openSettings } from "../../lib/tauri";
import { useEscape } from "../../lib/useEscape";
import { fixableInSettings } from "./actions";
import { diffWords } from "./diff";
import { PanelState, Provider, PROVIDER_META, STYLE_LABELS, Style } from "./types";

interface Props {
  provider: Provider;
  /** The digit key that uses this panel's result. */
  shortcut: number;
  state: PanelState;
  originalText: string;
  highlightDiffs: boolean;
  onUse: (text: string) => void;
  onCopy: (text: string) => void;
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
      return "przetwarzanie";
    case "streaming":
      return "pisze";
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
  shortcut,
  state,
  originalText,
  highlightDiffs,
  onUse,
  onCopy,
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
    // Nothing asked yet: the grid shows one shared hint, the panel stays quiet.
    if (state.status === "idle") return null;
    if (state.status === "error") {
      return (
        <div className="panel__error" role="alert">
          <strong>Zapytanie nie powiodło się</strong>
          <p>{state.error}</p>
        </div>
      );
    }
    if (state.status === "cancelled") {
      return <p className="panel__note">Przerwane przed odpowiedzią. Ponów tylko ten panel.</p>;
    }
    if (busy && state.text === "") {
      return (
        <div className="panel__skeleton" role="status" aria-label="Ładowanie">
          <span />
          <span />
          <span />
        </div>
      );
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
      <header className="panel__head">
        {busy && (
          <div
            className="panel__progress"
            role="progressbar"
            aria-label="Trwa przetwarzanie"
          >
            <div className="panel__progress-bar" />
          </div>
        )}
        <span className="panel__title">
          <span className="panel__dot" />
          {meta.label}
        </span>
        <span className="panel__head-right">
          {state.status !== "idle" && (
            <span className={`panel__status panel__status--${state.status}`}>
              {state.status === "error" && <Icon name="alert" size={13} />}
              {state.status === "done" && <Icon name="check" size={13} />}
              {statusLabel(state)}
            </span>
          )}
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
          {/* The menu keeps the right edge in every state, so it stays under the
              cursor when a panel finishes; cancel and like share the slot before it. */}
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
          {/* focus leaving the button and its list closes the menu (click elsewhere, Tab) */}
          <span
            className="panel__menu-wrap"
            onBlur={(e) => {
              if (!e.currentTarget.contains(e.relatedTarget)) setMenuOpen(false);
            }}
          >
            <button
              data-ux={`menu-${provider}`}
              className="icon-btn"
              title="Więcej akcji"
              aria-label={`Więcej akcji: ${meta.label}`}
              aria-expanded={menuOpen}
              disabled={state.status === "idle"}
              onClick={() => setMenuOpen((o) => !o)}
            >
              <Icon name="more" />
            </button>
            {/* a plain group of buttons: no ARIA menu role, which would promise
                arrow-key navigation this list does not have */}
            {menuOpen && (
              <div className="panel__menu" onMouseLeave={() => setMenuOpen(false)}>
                {canUse && (
                  <button
                    className="panel__menu-first"
                    onClick={() => {
                      setMenuOpen(false);
                      onCopy(state.text);
                    }}
                  >
                    Kopiuj tekst
                    <Icon name="copy" size={14} />
                  </button>
                )}
                <span className="panel__menu-label">Przetwórz ponownie jako</span>
                {REPROCESS_STYLES.map((s) => (
                  <button
                    key={s}
                    onClick={() => {
                      setMenuOpen(false);
                      onReprocess(provider, s);
                    }}
                  >
                    {STYLE_LABELS[s]}
                    {/* the style this panel's text is in now */}
                    {state.status === "done" && state.style === s && (
                      <Icon name="check" size={14} />
                    )}
                  </button>
                ))}
              </div>
            )}
          </span>
        </span>
      </header>

      <div className="panel__body">{body()}</div>

      {/* No result yet: no footer. It appears, always at the panel's bottom edge,
          once there is something to use or retry. */}
      {failed && (
        <div className="panel__foot">
          <button className="panel__use panel__use--retry" onClick={() => onRetry(provider)}>
            <Icon name="retry" /> Ponów {meta.label}
          </button>
          {/* a missing or rejected key, or an unknown model, is fixed in settings */}
          {fixableInSettings(state.error) && (
            <button className="panel__use panel__use--retry" onClick={() => openSettings()}>
              <Icon name="settings" /> Ustawienia
            </button>
          )}
        </div>
      )}
      {state.status === "done" && (
        <div className="panel__foot">
          <button
            className="panel__use"
            title={`Wklej wersję ${meta.label} (klawisz ${shortcut})`}
            disabled={!canUse}
            onClick={() => onUse(state.text)}
          >
            Użyj {meta.label}
            <kbd>{shortcut}</kbd>
          </button>
        </div>
      )}
    </section>
  );
}
