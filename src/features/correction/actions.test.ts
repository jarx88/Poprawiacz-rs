import { describe, expect, it } from "vitest";
import { isKeyError, keyAction } from "./actions";

const key = (key: string, mods: Partial<KeyboardEvent> = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  repeat: false,
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
  });
});

describe("isKeyError", () => {
  it("recognises a missing or rejected key, nothing else", () => {
    expect(isKeyError("Brak klucza API")).toBe(true);
    expect(isKeyError("response error (anthropic, status Some(401)): invalid x-api-key")).toBe(true);
    expect(isKeyError("response error (gemini, status Some(503)): overloaded")).toBe(false);
    expect(isKeyError("timeout (openai) after 35s")).toBe(false);
    expect(isKeyError(null)).toBe(false);
  });
});
