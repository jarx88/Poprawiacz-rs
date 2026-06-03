import { describe, expect, it } from "vitest";
import {
  SettingsForm,
  isValidStyle,
  validateApiKey,
  validateSettings,
} from "./validation";

describe("validateApiKey", () => {
  it("accepts empty keys (means leave unchanged)", () => {
    expect(validateApiKey("openai", "")).toBeNull();
    expect(validateApiKey("openai", "   ")).toBeNull();
  });

  it("flags too-short keys", () => {
    expect(validateApiKey("openai", "sk-1")).toMatch(/zbyt krótki/);
  });

  it("checks provider-specific prefixes", () => {
    expect(validateApiKey("openai", "sk-proj-abcdefghijk")).toBeNull();
    expect(validateApiKey("anthropic", "sk-ant-abcdefghijk")).toBeNull();
    expect(validateApiKey("anthropic", "sk-abcdefghijk")).toMatch(/sk-ant-/);
    expect(validateApiKey("gemini", "AIzaSyABCDEFGHIJK")).toBeNull();
    expect(validateApiKey("gemini", "xyzabcdefghijk")).toMatch(/AIza/);
  });
});

describe("isValidStyle", () => {
  it("accepts known styles", () => {
    expect(isValidStyle("normal")).toBe(true);
    expect(isValidStyle("translate_en")).toBe(true);
  });
  it("rejects unknown styles", () => {
    expect(isValidStyle("nope")).toBe(false);
  });
});

describe("validateSettings", () => {
  const base: SettingsForm = {
    models: {
      openai: "gpt-5-mini",
      anthropic: "claude-3-7-sonnet-latest",
      gemini: "gemini-2.5-flash",
      deepseek: "deepseek-chat",
    },
    defaultStyle: "normal",
    apiKeys: {},
  };

  it("passes a valid form", () => {
    expect(validateSettings(base).ok).toBe(true);
  });

  it("fails on empty model", () => {
    const r = validateSettings({
      ...base,
      models: { ...base.models, openai: "" },
    });
    expect(r.ok).toBe(false);
    expect(r.errors["model.openai"]).toBeDefined();
  });

  it("fails on unknown style", () => {
    const r = validateSettings({ ...base, defaultStyle: "bogus" });
    expect(r.ok).toBe(false);
    expect(r.errors.defaultStyle).toBeDefined();
  });

  it("fails on malformed api key", () => {
    const r = validateSettings({
      ...base,
      apiKeys: { gemini: "not-a-real-key" },
    });
    expect(r.ok).toBe(false);
    expect(r.errors["key.gemini"]).toBeDefined();
  });
});
