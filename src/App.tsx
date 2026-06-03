import { Suspense, lazy, useEffect, useState } from "react";
import { CorrectionView } from "./features/correction/CorrectionView";
import { onOpenSettings } from "./lib/tauri";

// Settings is heavier and rarely opened — lazy-load it so the shell renders fast.
const SettingsDialog = lazy(
  () => import("./features/settings/SettingsDialog"),
);

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Tray "Ustawienia" menu item opens the dialog.
  useEffect(() => {
    const un = onOpenSettings(() => setSettingsOpen(true));
    return () => {
      un.then((fn) => fn()).catch(() => {});
    };
  }, []);

  return (
    <div className="app">
      <header className="app__top">
        <strong>⌨️ Ctrl+Shift+C</strong>
        <span>— zaznacz tekst i naciśnij skrót, aby poprawić</span>
        <span style={{ flex: 1 }} />
        <button onClick={() => setSettingsOpen(true)}>⚙️ Ustawienia</button>
      </header>

      <CorrectionView />

      {settingsOpen && (
        <Suspense fallback={<div className="modal">Ładowanie…</div>}>
          <SettingsDialog onClose={() => setSettingsOpen(false)} />
        </Suspense>
      )}
    </div>
  );
}
