import { getCurrentWindow } from "@tauri-apps/api/window";

interface Props {
  onSettings: () => void;
}

/** Custom window chrome for the frameless window. The center area is a Tauri
 *  drag region; the right side has settings / minimize / close (hide to tray). */
export function Titlebar({ onSettings }: Props) {
  const win = getCurrentWindow();
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar__brand" data-tauri-drag-region>
        <span className="titlebar__dot" />
        <span className="titlebar__name">PoprawiaczTekstu</span>
        <span className="titlebar__hint">Ctrl+Shift+C</span>
      </div>
      <div className="titlebar__spacer" data-tauri-drag-region />
      <div className="titlebar__controls">
        <button className="tb-btn" title="Ustawienia" onClick={onSettings}>
          ⚙
        </button>
        <button className="tb-btn" title="Minimalizuj" onClick={() => win.minimize()}>
          ─
        </button>
        <button
          className="tb-btn tb-btn--close"
          title="Ukryj do zasobnika"
          onClick={() => win.hide()}
        >
          ✕
        </button>
      </div>
    </div>
  );
}
