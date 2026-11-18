/** What a key press in the main view means: a number is the index of the panel
 *  whose result to use (keys 1–4), "run" starts a correction (Ctrl+Enter). */
export function keyAction(
  e: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "repeat">,
  inField: boolean,
): number | "run" | null {
  if (e.repeat || e.altKey || e.metaKey) return null;
  if (e.ctrlKey) return e.key === "Enter" ? "run" : null;
  if (inField) return null; // digits typed into a text field are just text
  const i = ["1", "2", "3", "4"].indexOf(e.key);
  return i < 0 ? null : i;
}

/** Errors the user fixes in settings (missing or rejected key, unknown model).
 *  The backend words those so that they say "w ustawieniach"
 *  (`AiError::user_message` in the core crate). */
export function fixableInSettings(message: string | null): boolean {
  return (message ?? "").includes("w ustawieniach");
}
