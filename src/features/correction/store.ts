import { create } from "zustand";
import {
  CancelEvent,
  ChunkEvent,
  ErrorEvent,
  PanelState,
  PROVIDERS,
  Provider,
  ResultEvent,
} from "./types";

function allPanels(status: PanelState["status"]): Record<Provider, PanelState> {
  return Object.fromEntries(
    PROVIDERS.map((p) => [
      p,
      { status, text: "", error: null, elapsedMs: null, style: "normal", attempt: 0 } as PanelState,
    ]),
  ) as Record<Provider, PanelState>;
}

export interface CorrectionState {
  sessionId: number;
  originalText: string;
  panels: Record<Provider, PanelState>;

  /** Begin a new session; every panel resets to loading. A repeated start of
   *  the session already shown changes nothing. */
  startSession: (sessionId: number, text: string) => void;
  /** Append a streaming delta (ignored if stale). */
  applyChunk: (e: ChunkEvent) => void;
  /** Finalize a provider result (ignored if stale). */
  applyResult: (e: ResultEvent) => void;
  /** Record a provider error (ignored if stale). */
  applyError: (e: ErrorEvent) => void;
  /** Mark a provider cancelled (ignored if stale). */
  applyCancelled: (e: CancelEvent) => void;
  /** A single panel is being reprocessed: reset it to loading. */
  applyRestarted: (e: CancelEvent) => void;
  reset: () => void;
}

/** Count of panels that have finished (done/error/cancelled) — for "X/4". */
export function completedCount(panels: Record<Provider, PanelState>): number {
  return PROVIDERS.filter((p) =>
    ["done", "error", "cancelled"].includes(panels[p].status),
  ).length;
}

/** From another session, or from a run of this panel that a reprocess has
 *  since replaced: both share nothing with what the panel shows now. */
function stale(s: CorrectionState, e: CancelEvent): boolean {
  return (
    e.session_id !== s.sessionId || (e.attempt ?? Infinity) < s.panels[e.provider].attempt
  );
}

export const useCorrectionStore = create<CorrectionState>((set, get) => ({
  sessionId: 0,
  originalText: "",
  panels: allPanels("idle"),

  startSession: (sessionId, text) => {
    // "Popraw" starts the session from the backend event and from the command's
    // answer, whichever comes first; the second must not undo a fast failure.
    if (sessionId === get().sessionId) return;
    set({ sessionId, originalText: text, panels: allPanels("loading") });
  },

  applyChunk: (e) => {
    if (stale(get(), e)) return;
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
    if (stale(get(), e)) return;
    // anti-duplication: a finalized panel is not overwritten by a late result
    if (get().panels[e.provider].status === "done") return;
    set((s) => ({
      panels: {
        ...s.panels,
        [e.provider]: {
          ...s.panels[e.provider],
          status: "done",
          text: e.text,
          error: null,
          elapsedMs: e.elapsed_ms,
          style: e.style,
        },
      },
    }));
  },

  applyCancelled: (e) => {
    if (stale(get(), e)) return;
    set((s) => {
      const prev = s.panels[e.provider];
      if (prev.status === "done" || prev.status === "error") return s;
      return {
        panels: {
          ...s.panels,
          [e.provider]: { ...prev, status: "cancelled", error: null },
        },
      };
    });
  },

  applyRestarted: (e) => {
    if (stale(get(), e)) return;
    set((s) => ({
      panels: {
        ...s.panels,
        [e.provider]: {
          ...s.panels[e.provider],
          status: "loading",
          text: "",
          error: null,
          elapsedMs: null,
          attempt: e.attempt ?? s.panels[e.provider].attempt,
        },
      },
    }));
  },

  applyError: (e) => {
    if (stale(get(), e)) return;
    set((s) => ({
      panels: {
        ...s.panels,
        [e.provider]: {
          ...s.panels[e.provider],
          status: "error",
          text: "",
          error: e.message,
          elapsedMs: null,
        },
      },
    }));
  },

  reset: () => set({ sessionId: 0, originalText: "", panels: allPanels("idle") }),
}));
