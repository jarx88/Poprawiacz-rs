import { Suspense, lazy, useState } from "react";
import { CorrectionView } from "./features/correction/CorrectionView";

// Settings is heavier and rarely opened — lazy-load it so the shell renders fast.
const SettingsDialog = lazy(
  () => import("./features/settings/SettingsDialog"),
);

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);

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
