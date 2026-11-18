import { Icon } from "../../Icon";
import { writeClipboard } from "../../lib/tauri";
import { useEscape } from "../../lib/useEscape";

interface Props {
  text: string;
  onClose: () => void;
}

export function OriginalTextModal({ text, onClose }: Props) {
  useEscape(onClose);
  return (
    <div className="modal" role="dialog" aria-modal="true" aria-label="Oryginalny tekst">
      <div className="modal__card">
        <header className="modal__head">
          <h2>Oryginalny tekst</h2>
          <button className="icon-btn" title="Zamknij" aria-label="Zamknij" onClick={onClose}>
            <Icon name="close" />
          </button>
        </header>
        <div className="modal__body">
          <textarea className="original__text" readOnly value={text} />
        </div>
        <footer className="modal__actions">
          <button onClick={() => writeClipboard(text)}>
            <Icon name="copy" /> Kopiuj
          </button>
          <span className="spacer" />
          <button className="primary" autoFocus onClick={onClose}>
            Zamknij
          </button>
        </footer>
      </div>
    </div>
  );
}
