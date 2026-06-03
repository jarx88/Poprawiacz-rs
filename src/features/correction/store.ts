import { create } from "zustand";
import {
  ChunkEvent,
  ErrorEvent,
  PanelState,
  PROVIDERS,
  Provider,
  ResultEvent,
} from "./types";

function idlePanels(): Record<Provider, PanelState> {
  return Object.fromEntries(
    PROVIDERS.map((p) => [
      p,
      { status: "idle", text: "", error: null, elapsedMs: null } as PanelState,
    ]),
  ) as Record<Provider, PanelState>;
}

export interface CorrectionState {
  sessionId: number;
  originalText: string;
  panels: Record<Provider, PanelState>;

  /** Begin a new session; every panel resets to loading. */
  startSession: (sessionId: number, text: string) => void;
  /** Append a streaming delta (ignored if from a stale session). */
  applyChunk: (e: ChunkEvent) => void;
  /** Finalize a provider result (ignored if from a stale session). */
  applyResult: (e: ResultEvent) => void;
  /** Record a provider error (ignored if from a stale session). */
  applyError: (e: ErrorEvent) => void;
  reset: () => void;
}

export const useCorrectionStore = create<CorrectionState>((set, get) => ({
  sessionId: 0,
  originalText: "",
  panels: idlePanels(),

  startSession: (sessionId, text) =>
    set(() => ({
      sessionId,
      originalText: text,
      panels: Object.fromEntries(
        PROVIDERS.map((p) => [
          p,
          {
            status: "loading",
            text: "",
            error: null,
            elapsedMs: null,
          } as PanelState,
        ]),
      ) as Record<Provider, PanelState>,
    })),

  applyChunk: (e) => {
    if (e.session_id !== get().sessionId) return; // stale session guard
    set((s) => {
      const prev = s.panels[e.provider];
      if (prev.status === "done" || prev.status === "error") return s;
      return {
        panels: {
          ...s.panels,
          [e.provider]: {
            ...prev,
            status: "streaming",
            text: prev.text + e.delta,
          },
        },
      };
    });
  },

  applyResult: (e) => {
    if (e.session_id !== get().sessionId) return; // stale session guard
    set((s) => ({
      panels: {
        ...s.panels,
        [e.provider]: {
          status: "done",
          text: e.text,
          error: null,
          elapsedMs: e.elapsed_ms,
        },
      },
    }));
  },

  applyError: (e) => {
    if (e.session_id !== get().sessionId) return; // stale session guard
    set((s) => ({
      panels: {
        ...s.panels,
        [e.provider]: {
          status: "error",
          text: "",
          error: e.message,
          elapsedMs: null,
        },
      },
    }));
  },

  reset: () => set({ sessionId: 0, originalText: "", panels: idlePanels() }),
}));
