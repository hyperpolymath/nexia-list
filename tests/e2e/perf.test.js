// SPDX-License-Identifier: MPL-2.0
//
// Browser performance benchmark: a 10,000-note / 50,000-link notebook in
// the real app (AffineScript UI + WASM core) in headless Chromium.
//
// Measures and gates:
//   * cold start with the large notebook restored from IndexedDB;
//   * substring search (a 200-id page + total, as the sidebar asks) and
//     backlink queries in the browser's WASM (p95 < 10 ms);
//   * continuous canvas pan and wheel zoom driven at display rate (one input
//     per animation frame) with ~300 cards on screen: the work per frame
//     (event handling + update + render) must fit a 60 FPS budget (p95 < 16.7 ms).
// Frame intervals are reported too, but headless Chromium paces frames
// without a display, so they are not used as the gate.
//
// Run: bun run build && cd tests/e2e && bun install --frozen-lockfile && bun test --timeout 600000 perf.test.js

import { afterAll, beforeAll, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { freshProfile, open, serve } from "./harness.js";
import init, { WasmNotebook } from "../../web/wasm/nexia_core.js";

const NOTES = 10_000;
const LINKS = 50_000;
const FRAME_BUDGET_MS = 1000 / 60;
const QUERY_BUDGET_MS = 10;

let server;
let session;
let notebookJson;

/** Deterministic PRNG (mulberry32) so every run measures the same graph. */
function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const WORDS =
  "graph note spatial canvas agent prototype link idea draft review garden atlas memory context outline theme signal pattern lambda delta field query archive sketch thread summary source".split(
    " ",
  );

/** Build the benchmark notebook with the real WASM core; returns its JSON. */
function buildNotebook() {
  const rand = rng(0xc0ffee);
  const nb = new WasmNotebook("Benchmark");
  const ids = [];
  for (let i = 0; i < NOTES; i++) {
    const note = nb.create_note_at(
      `Note ${i} ${WORDS[i % WORDS.length]}`,
      (i % 100) * 240,
      Math.floor(i / 100) * 180,
    );
    const words = Array.from(
      { length: 40 },
      () => WORDS[Math.floor(rand() * WORDS.length)],
    );
    nb.update_content(note.id, words.join(" "));
    if (i % 10 === 0)
      nb.setComputedField(note.id, "words", "(count (words (content self)))");
    ids.push(note.id);
  }
  let made = 0;
  while (made < LINKS) {
    const from = ids[Math.floor(rand() * NOTES)];
    const to = ids[Math.floor(rand() * NOTES)];
    if (from === to) continue;
    try {
      nb.link(from, to);
      made++;
    } catch {
      // Already linked: draw again.
    }
  }
  return nb.to_json();
}

/** p50/p95/max of a list of milliseconds. */
function stats(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const at = (q) => s[Math.min(s.length - 1, Math.round((s.length - 1) * q))];
  return { p50: at(0.5), p95: at(0.95), max: s[s.length - 1], n: s.length };
}

const fmt = (st) =>
  `p50 ${st.p50.toFixed(2)} ms · p95 ${st.p95.toFixed(2)} ms · max ${st.max.toFixed(2)} ms (n=${st.n})`;

beforeAll(async () => {
  await init({
    module_or_path: await readFile(
      new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
    ),
  });
  const t0 = performance.now();
  notebookJson = buildNotebook();
  console.log(
    `generated ${NOTES} notes / ${LINKS} links in ${(performance.now() - t0).toFixed(0)} ms (${(notebookJson.length / 2 ** 20).toFixed(1)} MiB JSON)`,
  );

  server = serve();
  const url = `http://localhost:${server.port}/`;
  const profile = freshProfile();
  // Seed IndexedDB through the app's origin, then relaunch so the app
  // restores the large notebook on a cold start.
  const seed = await open(url, profile);
  await seed.page.evaluate(
    (json) =>
      new Promise((resolve, reject) => {
        const req = indexedDB.open("nexia", 1);
        req.onupgradeneeded = () => req.result.createObjectStore("kv");
        req.onsuccess = () => {
          const tx = req.result.transaction("kv", "readwrite");
          tx.objectStore("kv").put(json, "nexia.autosave");
          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        };
      }),
    notebookJson,
  );
  await seed.context.close();
  session = await open(url, profile);
});

afterAll(async () => {
  await session?.context.close();
  server?.stop(true);
});

test("cold start restores the 10k-note notebook", async () => {
  const { page } = session;
  const ms = await page.evaluate(() => performance.now());
  const coreMs = Number(
    await page.evaluate(() => document.documentElement.dataset.bootMs),
  );
  console.log(
    `cold start with ${NOTES} notes: ${ms.toFixed(0)} ms to interactive (WASM load + restore: ${coreMs} ms)`,
  );
  expect(await page.textContent(".note-count")).toBe(`${NOTES} notes`);
  expect(await page.textContent(".list-more")).toContain("search to narrow");
});

test("substring search and backlinks in the browser's WASM stay under 10 ms", async () => {
  const { page } = session;
  const result = await page.evaluate(() => {
    const queries = [
      "garden",
      "Note 42",
      "lambda delta",
      "zzz-no-match",
      "NOTE 9999",
    ];
    const notes = globalThis.nx_notes();
    const search = [];
    const backlinks = [];
    for (let i = 0; i < 100; i++) {
      let t = performance.now();
      globalThis.nx_search(queries[i % queries.length], 200);
      search.push(performance.now() - t);
      t = performance.now();
      globalThis.nx_backlinks(notes[(i * 7919) % notes.length].id);
      backlinks.push(performance.now() - t);
    }
    return { search, backlinks };
  });
  const s = stats(result.search);
  const b = stats(result.backlinks);
  console.log(`search (WASM, ${NOTES} notes):    ${fmt(s)}`);
  console.log(`backlinks (WASM, ${LINKS} links): ${fmt(b)}`);
  expect(s.p95).toBeLessThan(QUERY_BUDGET_MS);
  expect(b.p95).toBeLessThan(QUERY_BUDGET_MS);
});

/**
 * Drive `frames` animation frames, calling `input(i)` once per frame, and
 * record the work each frame does (the time the event takes to handle plus
 * the runtime's render callback) and the frame intervals.
 */
async function drive(page, frames, input) {
  return page.evaluate(
    ({ frames, inputSrc }) =>
      new Promise((resolve) => {
        const input = new Function("i", inputSrc);
        const work = [];
        const intervals = [];
        // Time every rAF callback the runtime schedules (renders).
        const raf = window.requestAnimationFrame.bind(window);
        let renderMs = 0;
        window.requestAnimationFrame = (cb) =>
          raf((t) => {
            const t0 = performance.now();
            cb(t);
            renderMs += performance.now() - t0;
          });
        let last = null;
        let i = 0;
        const tick = (t) => {
          if (last !== null) intervals.push(t - last);
          last = t;
          if (i > 0) work.push(renderMs);
          if (i === frames) {
            window.requestAnimationFrame = raf;
            resolve({ work, intervals });
            return;
          }
          const t0 = performance.now();
          input(i);
          renderMs = performance.now() - t0;
          i++;
          raf(tick);
        };
        raf(tick);
      }),
    { frames, inputSrc: input },
  );
}

test("continuous pan and zoom over a dense canvas fit a 60 FPS frame budget", async () => {
  const { page } = session;
  await page.click("button:text-is('Canvas')");
  await page.waitForSelector(".canvas-note");
  // Zoom out so a few hundred cards are on screen.
  for (let i = 0; i < 12; i++)
    await page.click("button[aria-label='Zoom out']");
  await page.waitForFunction(
    () => Number(document.querySelector(".canvas-view")?.dataset.visible) > 150,
  );
  const visible = Number(
    await page.$eval(".canvas-view", (e) => e.dataset.visible),
  );

  const box = await page.locator(".canvas-view").boundingBox();
  const cx = Math.round(box.x + box.width - 40);
  const cy = Math.round(box.y + box.height - 40);
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  const pan = await drive(
    page,
    180,
    `window.dispatchEvent(new PointerEvent("pointermove", { clientX: ${cx} - (i % 60) * 6, clientY: ${cy} - (i % 60) * 4, bubbles: true }));`,
  );
  await page.mouse.up();
  const zoom = await drive(
    page,
    120,
    `document.querySelector(".canvas-view").dispatchEvent(new WheelEvent("wheel", { deltaY: i % 40 < 20 ? -40 : 40, clientX: 600, clientY: 400, bubbles: true, cancelable: true }));`,
  );

  const pw = stats(pan.work);
  const zw = stats(zoom.work);
  const pi = stats(pan.intervals);
  console.log(`canvas with ${visible} visible cards of ${NOTES}:`);
  console.log(`  pan  work/frame: ${fmt(pw)}`);
  console.log(`  zoom work/frame: ${fmt(zw)}`);
  console.log(
    `  pan frame interval (headless pacing, informational): ${fmt(pi)} ≈ ${(1000 / pi.p50).toFixed(0)} FPS`,
  );
  expect(visible).toBeGreaterThan(150);
  expect(pw.p95).toBeLessThan(FRAME_BUDGET_MS);
  expect(zw.p95).toBeLessThan(FRAME_BUDGET_MS);
  expect(session.errors).toEqual([]);
});
