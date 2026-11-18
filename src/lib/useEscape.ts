import { useEffect, useEffectEvent } from "react";

/** Close a dialog or menu with Escape. */
export function useEscape(onEscape: () => void) {
  const handler = useEffectEvent(onEscape);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handler();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
