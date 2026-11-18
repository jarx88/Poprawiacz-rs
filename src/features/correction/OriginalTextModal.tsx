import { Icon } from "../../Icon";
import { writeClipboard } from "../../lib/tauri";
import { useModal } from "../../lib/useModal";

interface Props {
  text: string;
  onClose: () => void;
}

export function OriginalTextModal({ text, onClose }: Props) {
  const { ref, close } = useModal();
  return (
    <dialog ref={ref} className="modal" aria-label="Oryginalny tekst" onClose={onClose}>
      <div className="modal__card">
        <header className="modal__head">
          <h2>Oryginalny tekst</h2>
          <button className="icon-btn" title="Zamknij" aria-label="Zamknij" onClick={close}>
            <Icon name="close" />
          </button>
        </header>
        <div className="modal__body">
          <textarea
            className="original__text"
            aria-label="Oryginalny tekst"
            readOnly
            data-focus
            value={text}
          />
        </div>
        <footer className="modal__actions">
          <button onClick={() => writeClipboard(text)}>
            <Icon name="copy" /> Kopiuj
          </button>
          <span className="spacer" />
          <button className="primary" onClick={close}>
            Zamknij
          </button>
        </footer>
      </div>
    </dialog>
  );
}
