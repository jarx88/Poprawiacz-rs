import { describe, expect, it } from "vitest";
import { fixableInSettings, keyAction } from "./actions";

const key = (key: string, mods: Partial<KeyboardEvent> = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
  repeat: false,
  isComposing: false,
  ...mods,
});

describe("keyAction", () => {
  it("maps 1–4 to a panel index outside text fields only", () => {
    expect(keyAction(key("1"), false)).toBe(0);
    expect(keyAction(key("4"), false)).toBe(3);
    expect(keyAction(key("5"), false)).toBeNull();
    expect(keyAction(key("2"), true)).toBeNull();
  });

  it("runs on Ctrl+Enter, also inside a text field", () => {
    expect(keyAction(key("Enter", { ctrlKey: true }), true)).toBe("run");
    expect(keyAction(key("Enter"), false)).toBeNull();
  });

  it("ignores modified digits and key repeat", () => {
    expect(keyAction(key("1", { ctrlKey: true }), false)).toBeNull();
    expect(keyAction(key("1", { altKey: true }), false)).toBeNull();
    expect(keyAction(key("1", { repeat: true }), false)).toBeNull();
    expect(keyAction(key("1", { isComposing: true }), false)).toBeNull();
    expect(keyAction(key("Enter", { ctrlKey: true, shiftKey: true }), false)).toBeNull();
    expect(keyAction(key("1", { shiftKey: true }), false)).toBeNull(); // would paste as Ctrl+Shift+V
  });
});

describe("fixableInSettings", () => {
  // the strings are the backend's own (AiError::user_message, modules/ai.rs)
  it("recognises errors that settings can fix, nothing else", () => {
    expect(fixableInSettings("Brak klucza API. Dodaj go w ustawieniach.")).toBe(true);
    expect(
      fixableInSettings(
        "Dostawca odrzucił klucz API. Sprawdź klucz w ustawieniach. Szczegóły (HTTP 401): invalid x-api-key",
      ),
    ).toBe(true);
    expect(
      fixableInSettings("Dostawca nie zna tego modelu. Sprawdź nazwę modelu w ustawieniach. (HTTP 404)"),
    ).toBe(true);
    expect(
      fixableInSettings("Serwer dostawcy ma awarię albo jest przeciążony. Spróbuj za chwilę. (HTTP 503)"),
    ).toBe(false);
    expect(fixableInSettings("Dostawca nie odpowiedział w ciągu 35 s. Spróbuj ponownie.")).toBe(false);
    expect(fixableInSettings(null)).toBe(false);
  });
});
