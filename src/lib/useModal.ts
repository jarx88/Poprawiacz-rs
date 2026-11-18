import { useLayoutEffect, useRef } from "react";

/** A native modal <dialog>: the page behind is inert, Tab stays inside, Escape
 *  closes only the top dialog, and close() returns focus to the opener.
 *  Put `ref` and `onClose` on the <dialog> and close it with `close()`. */
export function useModal() {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = ref.current;
    if (!dialog || dialog.open) return;
    dialog.showModal();
    // showModal focuses the first control; a dialog may name a better one
    dialog.querySelector<HTMLElement>("[data-focus]")?.focus();
  }, []);
  return { ref, close: () => ref.current?.close() };
}
