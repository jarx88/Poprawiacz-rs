#!/usr/bin/env node
// Usage: node ux-climb/capture.mjs <round>
// Writes ux-climb/round-<round>/<state>.png and metrics.json (contrast, clipping).
// Runs its own Vite on port 5199, so the dev environment on 5173 is not touched.
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const round = process.argv[2];
if (!round) throw new Error("usage: capture.mjs <round>");
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "ux-climb", `round-${round}`);
const VITE = 5199;
const base = `http://localhost:${VITE}/ux-climb/harness.html`;

// state -> window size: the default window, plus the minimum window for the long text
const STATES = {
  idle: [1100, 760],
  loading: [1100, 760],
  mixed: [1100, 760],
  done: [1100, 760],
  errors: [1100, 760],
  menu: [1100, 760],
  settings: [1100, 760],
  original: [1100, 760],
  long: [720, 520],
  // scrolled to the bottom: the scroll fade must be gone (not shown to judges)
  "settings-end": [1100, 760],
  "long-end": [720, 520],
  // presses 2 and Ctrl+Enter; the harness throws if neither reaches the backend
  "done-keys": [1100, 760],
  // an open panel menu owns the keyboard: the same keys must do nothing
  "menu-keys": [1100, 760],
  // lower-row menu in a short window (opens upwards), notice chip in the narrow one
  "menu-low": [1100, 660],
  "errors-narrow": [720, 520],
  // a failure notice stays until the next session; a menu closes when focus leaves it
  "idle-notice": [1100, 760],
  "menu-blur": [1100, 760],
  // the dialogs are native: focus handling, and Escape as a real key press
  "original-focus": [1100, 760],
  "original-esc": [1100, 760],
  "settings-esc": [1100, 760],
};

mkdirSync(out, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "ux-climb-"));
const kids = [
  spawn("pnpm", ["exec", "vite", "--port", VITE, "--strictPort"], { cwd: root, stdio: "ignore" }),
  // Port 0: Chrome picks a free one and writes it to its profile, so this script
  // can only ever drive the browser it started, never someone else's.
  spawn(
    "google-chrome",
    ["--headless=new", "--no-first-run", "--disable-gpu", "--hide-scrollbars=false",
      "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
    { stdio: "ignore", env: { ...process.env, TZ: "Europe/Warsaw" } },
  ),
];
// Vite exits at once when its port is taken; without this the script would
// screenshot whatever else is listening there.
kids[0].on("exit", () => {
  console.error(`vite exited early (is port ${VITE} taken?)`);
  process.exit(1);
});
const cleanup = () => {
  kids.forEach((k) => k.kill());
  try {
    // Chrome needs a moment to let go of its profile after SIGTERM.
    rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  } catch {}
};
process.on("exit", cleanup);
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => process.exit(1));

async function retry(fn) {
  for (let i = 0; ; i++) {
    try {
      return await fn();
    } catch (e) {
      if (i > 60) throw e;
      await new Promise((r) => setTimeout(r, 500));
    }
  }
}
const ok = async (url) => {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: ${r.status}`);
  return r;
};

await retry(() => ok(base));
// Chrome may be caught mid-write or before its first page exists: retry both.
const cdpPort = await retry(() => {
  const port = readFileSync(join(profile, "DevToolsActivePort"), "utf8").split("\n")[0];
  if (!/^\d+$/.test(port)) throw new Error("DevToolsActivePort not written yet");
  return port;
});
const target = await retry(async () => {
  const pages = await (await ok(`http://localhost:${cdpPort}/json/list`)).json();
  const page = pages.find((t) => t.type === "page");
  if (!page) throw new Error("no page target yet");
  return page;
});
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r, j) => ((ws.onopen = r), (ws.onerror = j)));
let seq = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (!pending.has(msg.id)) return;
  const { resolve, reject } = pending.get(msg.id);
  pending.delete(msg.id);
  msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
};
const cdp = (method, params = {}) =>
  new Promise((resolve, reject) => {
    pending.set(++seq, { resolve, reject });
    ws.send(JSON.stringify({ id: seq, method, params }));
  });
const evaluate = async (expression) => {
  const r = await cdp("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? "eval failed");
  return r.result.value;
};

const metrics = {};
// a headless page has no focus, so focus and blur events would never fire
await cdp("Emulation.setFocusEmulationEnabled", { enabled: true });
for (const [state, [width, height]] of Object.entries(STATES)) {
  await cdp("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await cdp("Page.navigate", { url: `${base}?state=${state}` });
  try {
    await retry(() => evaluate("window.__uxReady.then(() => true)"));
  } catch (e) {
    const where = await evaluate("location.href + ' | ' + document.body.innerText.slice(0, 300)");
    throw new Error(`state ${state} never became ready (${where})`, { cause: e });
  }
  if (state.endsWith("-esc")) {
    // sent through the browser: a native dialog ignores synthetic key events
    for (const type of ["keyDown", "keyUp"])
      await cdp("Input.dispatchKeyEvent", { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    const open = await evaluate(
      "new Promise((r) => setTimeout(() => r(document.querySelectorAll('dialog').length), 150))",
    );
    if (open) throw new Error(`Escape left a dialog open in ${state}`);
  }
  metrics[state] = await evaluate("window.__uxMetrics()");
  // let the last state change reach the screen before the shot
  await evaluate("new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(r, 100))))");
  const shot = await cdp("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(out, `${state}.png`), Buffer.from(shot.data, "base64"));
}
writeFileSync(join(out, "metrics.json"), JSON.stringify(metrics, null, 1) + "\n");
ws.close();
console.log(`captured: ${out}`);
process.exit(0);
