import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "./Icon";
import { BuildInfo, getAppBuild } from "./lib/tauri";
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
  const [build, setBuild] = useState<BuildInfo | null>(null);
  useEffect(() => {
    getAppBuild().then(setBuild, () => {});
  }, []);
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar__brand" data-tauri-drag-region>
        <span className="titlebar__name" data-tauri-drag-region>
          poprawiacz<span className="titlebar__name-dim">.tekstu</span>
        </span>
        <kbd title="Skrót: popraw zaznaczony tekst">Ctrl+Shift+C</kbd>
        {build && (
          <span className="titlebar__version" title={`build ${build.build}`}>
            v{build.version} · {formatBuiltAt(build.built_at)}
          </span>
        )}
      </div>
      <div className="titlebar__spacer" data-tauri-drag-region />
      <span className="titlebar__session">
        sesja {sessionId} · api {done}/4
      </span>
      <div className="titlebar__controls">
        <button
          className="tb-btn tb-btn--text"
          title="Ustawienia"
          aria-label="Ustawienia"
          onClick={onSettings}
        >
          <Icon name="settings" />
          <span>Ustawienia</span>
        </button>
        <button
          className="tb-btn"
          title="Minimalizuj"
          aria-label="Minimalizuj"
          onClick={() => win.minimize()}
        >
          <Icon name="minus" />
        </button>
        <button
          className="tb-btn tb-btn--close"
          title="Ukryj do zasobnika"
          aria-label="Ukryj do zasobnika"
          onClick={() => win.hide()}
        >
          <Icon name="close" />
        </button>
      </div>
    </div>
  );
}

/** Build time as "26.10, 20:48": the quickest way to tell builds apart. */
function formatBuiltAt(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString("pl-PL", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
