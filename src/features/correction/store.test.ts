import { beforeEach, describe, expect, it } from "vitest";
import { useCorrectionStore } from "./store";

const store = () => useCorrectionStore.getState();

beforeEach(() => {
  useCorrectionStore.getState().reset();
});

describe("correction store session isolation", () => {
  it("starts a session and sets all panels to loading", () => {
    store().startSession(1, "tekst");
    const s = store();
    expect(s.sessionId).toBe(1);
    expect(s.originalText).toBe("tekst");
    expect(s.panels.openai.status).toBe("loading");
    expect(s.panels.deepseek.status).toBe("loading");
  });

  it("accumulates streaming chunks for the current session", () => {
    store().startSession(2, "x");
    store().applyChunk({ session_id: 2, provider: "openai", delta: "abc" });
    store().applyChunk({ session_id: 2, provider: "openai", delta: "def" });
    expect(store().panels.openai.text).toBe("abcdef");
    expect(store().panels.openai.status).toBe("streaming");
  });

  it("drops chunks from a stale (older) session", () => {
    store().startSession(5, "x");
    store().applyChunk({ session_id: 4, provider: "openai", delta: "stale" });
    expect(store().panels.openai.text).toBe("");
    expect(store().panels.openai.status).toBe("loading");
  });

  it("drops results from a stale session so they cannot overwrite a newer one", () => {
    store().startSession(10, "x");
    store().applyResult({
      session_id: 9,
      provider: "anthropic",
      text: "OLD",
      elapsed_ms: 100,
    });
    expect(store().panels.anthropic.text).toBe("");
    expect(store().panels.anthropic.status).toBe("loading");
  });

  it("finalizes a result for the current session", () => {
    store().startSession(3, "x");
    store().applyResult({
      session_id: 3,
      provider: "gemini",
      text: "poprawione",
      elapsed_ms: 1200,
    });
    expect(store().panels.gemini.status).toBe("done");
    expect(store().panels.gemini.text).toBe("poprawione");
    expect(store().panels.gemini.elapsedMs).toBe(1200);
  });

  it("isolates provider failures from each other", () => {
    store().startSession(7, "x");
    store().applyError({
      session_id: 7,
      provider: "deepseek",
      message: "timeout",
    });
    store().applyResult({
      session_id: 7,
      provider: "openai",
      text: "ok",
      elapsed_ms: 50,
    });
    expect(store().panels.deepseek.status).toBe("error");
    expect(store().panels.deepseek.error).toBe("timeout");
    expect(store().panels.openai.status).toBe("done");
  });

  it("ignores chunks after a panel is finalized", () => {
    store().startSession(8, "x");
    store().applyResult({
      session_id: 8,
      provider: "openai",
      text: "final",
      elapsed_ms: 10,
    });
    store().applyChunk({ session_id: 8, provider: "openai", delta: "late" });
    expect(store().panels.openai.text).toBe("final");
    expect(store().panels.openai.status).toBe("done");
  });
});
