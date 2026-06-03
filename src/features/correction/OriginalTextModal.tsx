import { writeClipboard } from "../../lib/tauri";

interface Props {
  text: string;
  onClose: () => void;
}

export function OriginalTextModal({ text, onClose }: Props) {
  return (
    <div className="modal" role="dialog" aria-label="Oryginalny tekst">
      <div className="modal__card">
        <h2>📄 Oryginalny tekst</h2>
        <textarea className="original__text" readOnly value={text} />
        <div className="modal__actions">
          <button onClick={() => writeClipboard(text)}>📋 Kopiuj</button>
          <span style={{ flex: 1 }} />
          <button className="primary" onClick={onClose}>
            Zamknij
          </button>
        </div>
      </div>
    </div>
  );
}
