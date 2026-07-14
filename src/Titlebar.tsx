import { getCurrentWindow } from "@tauri-apps/api/window";
import { completedCount, useCorrectionStore } from "./features/correction/store";

interface Props {
  onSettings: () => void;
}

/** Custom window chrome for the frameless window. The center area is a Tauri
 *  drag region; the right side has session info + settings / minimize / close. */
export function Titlebar({ onSettings }: Props) {
  const win = getCurrentWindow();
  const { sessionId, panels } = useCorrectionStore();
  const done = completedCount(panels);
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar__brand" data-tauri-drag-region>
        <span className="titlebar__name" data-tauri-drag-region>
          poprawiacz<span className="titlebar__name-dim">.tekstu</span>
        </span>
        <span className="titlebar__hint">Ctrl+Shift+C</span>
      </div>
      <div className="titlebar__spacer" data-tauri-drag-region />
      <span className="titlebar__session">
        sesja {sessionId} · api {done}/4
      </span>
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
