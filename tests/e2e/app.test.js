// SPDX-License-Identifier: MPL-2.0
//
// End-to-end tests for the Nexia-List web app (the AffineScript UI in
// ui/src on the affinescript-tea runtime, over the Rust/WASM core), driven
// in headless Chromium.
//
// Run: bun run build && cd tests/e2e && bun install --frozen-lockfile && bun test app.test.js

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { autosaved, eventually, freshProfile, open, serve } from "./harness.js";

let server;
let url;

beforeAll(() => {
  server = serve();
  url = `http://localhost:${server.port}/`;
});
afterAll(() => server?.stop(true));

/** Assert |a - b| <= 1 (pointer coordinates are integer CSS pixels). */
const near = (a, b) => expect(Math.abs(a - b)).toBeLessThanOrEqual(1);

/**
 * Click "+ New Note" and wait for its editor: renders happen on the next
 * animation frame, and filling before then would type into the previous
 * note's still-visible inputs.
 */
async function newNote(page) {
  const before = await page.$$eval(".note-list li", (els) => els.length);
  await page.click(".sidebar-header .btn-primary");
  await page.waitForFunction(
    (n) => document.querySelectorAll(".note-list li").length === n + 1,
    before,
  );
  await page.waitForFunction(
    () => document.querySelector(".note-title-input")?.value === "New Note",
  );
}

/** Titles of the notes in the sidebar, in order. */
const sidebarTitles = (page) =>
  page.$$eval(".note-list .note-title", (els) => els.map((e) => e.textContent));

/** Canvas cards: [{ title, left, top }]. */
const cards = (page) =>
  page.$$eval(".canvas-note", (els) =>
    els.map((e) => ({
      id: e.dataset.id,
      title: e.querySelector(".canvas-note-title")?.textContent ?? null,
      left: parseFloat(e.style.left),
      top: parseFloat(e.style.top),
    })),
  );

describe("cold start", () => {
  test("boots to an interactive app in under one second", async () => {
    const { context, page, errors } = await open(url, freshProfile());
    // Time from navigation start to the booted toolbar, measured in the page.
    const ms = await page.evaluate(() => performance.now());
    const coreMs = Number(
      await page.evaluate(() => document.documentElement.dataset.bootMs),
    );
    console.log(
      `cold start: ${ms.toFixed(0)} ms to interactive (WASM core + restore: ${coreMs} ms)`,
    );
    // The brief's budget is 1 s; COLD_START_BUDGET_MS lets a slower machine
    // run the suite without changing what is asserted by default.
    expect(ms).toBeLessThan(Number(process.env.COLD_START_BUDGET_MS ?? 1000));
    expect(await page.textContent(".sidebar h2")).toBe("Untitled Notebook");
    expect(errors).toEqual([]);
    await context.close();
  });
});

describe("spatial canvas", () => {
  let session;
  beforeAll(async () => {
    session = await open(url, freshProfile());
    await session.page.click("button:text-is('Canvas')");
    await session.page.waitForSelector(".canvas-view");
  });
  afterAll(() => session.context.close());

  test("double-clicking the canvas creates a note where you clicked", async () => {
    const { page } = session;
    const box = await page.locator(".canvas-view").boundingBox();
    await page.mouse.dblclick(box.x + 300, box.y + 200);
    const [card] = await eventually(
      () => cards(page),
      (cs) => expect(cs.length).toBe(1),
    );
    near(card.left, 300);
    near(card.top, 200);
    // A new note opens for inline editing.
    expect(
      await page.$$eval(".canvas-note .card-title-input", (els) => els.length),
    ).toBe(1);
    await page.keyboard.press("Escape");
    await page.waitForSelector(".canvas-note .canvas-note-title");
  });

  test("dragging a note moves it by the pointer delta", async () => {
    const { page } = session;
    const [before] = await cards(page);
    const handle = page.locator(`.canvas-note[data-id="${before.id}"]`);
    const b = await handle.boundingBox();
    await page.mouse.move(b.x + 10, b.y + 10);
    await page.mouse.down();
    await page.mouse.move(b.x + 60, b.y + 40, { steps: 5 });
    await page.mouse.move(b.x + 130, b.y + 90, { steps: 5 });
    await page.mouse.up();
    const [after] = await eventually(
      () => cards(page),
      ([c]) => near(c.left, before.left + 120),
    );
    near(after.top, before.top + 80);
  });

  test("double-clicking a note edits its title and text in place", async () => {
    const { page } = session;
    const [card] = await cards(page);
    await page.dblclick(
      `.canvas-note[data-id="${card.id}"] .canvas-note-title`,
    );
    const title = page.locator(
      `.canvas-note[data-id="${card.id}"] .card-title-input`,
    );
    await title.fill("Spatial idea");
    await page
      .locator(`.canvas-note[data-id="${card.id}"] .card-content-input`)
      .fill("near the [[Spatial idea]] cluster");
    await page.keyboard.press("Escape");
    await eventually(
      () => cards(page),
      ([c]) => expect(c.title).toBe("Spatial idea"),
    );
    expect(
      await page.textContent(
        `.canvas-note[data-id="${card.id}"] .canvas-note-preview`,
      ),
    ).toContain("cluster");
  });

  test("panning with the background and zooming with the wheel", async () => {
    const { page } = session;
    const transform = () => page.$eval(".canvas", (e) => e.style.transform);
    const box = await page.locator(".canvas-view").boundingBox();
    await page.mouse.move(box.x + 900, box.y + 600);
    await page.mouse.down();
    await page.mouse.move(box.x + 950, box.y + 630, { steps: 4 });
    await page.mouse.up();
    await eventually(transform, (t) =>
      expect(t).toContain("translate(50px, 30px)"),
    );
    await page.mouse.wheel(0, -100);
    await eventually(transform, (t) => expect(t).toContain("scale(1.1)"));
    await page.click("button[aria-label='Reset view']");
    await eventually(transform, (t) =>
      expect(t).toBe("translate(0px, 0px) scale(1)"),
    );
  });

  test("no runtime errors", () => {
    expect(session.errors).toEqual([]);
  });
});

describe("λδ computed fields", () => {
  let session;
  beforeAll(async () => {
    session = await open(url, freshProfile());
  });
  afterAll(() => session.context.close());

  test("a computed field evaluates through the WASM core and updates live", async () => {
    const { page } = session;
    await newNote(page);
    await page.fill(".note-title-input", "Draft");
    await page.fill(".note-content-input", "one two three");
    await page.fill(".field-name-input", "words");
    await page.fill(".formula-source", "(count (words (content self)))");
    // The formula previews live while it is typed...
    await eventually(
      () => page.textContent(".field-form .formula-result"),
      (v) => expect(v).toBe("3"),
    );
    await page.click("button:text-is('Add field')");
    const value = () => page.textContent(".field-list .field-value");
    await eventually(value, (v) => expect(v).toBe("3"));
    // ...and the stored field re-evaluates as the note changes.
    await page.fill(".note-content-input", "one two three four five");
    await eventually(value, (v) => expect(v).toBe("5"));
  });

  test("errors are shown per field, not thrown", async () => {
    const { page } = session;
    await page.fill(".field-name-input", "broken");
    await page.fill(".formula-source", "(car)");
    await page.click("button:text-is('Add field')");
    await eventually(
      () => page.$$eval(".field-list .field", (els) => els.length),
      (n) => expect(n).toBe(2),
    );
    expect(
      await page.$$eval(".field-list .field-error", (els) => els.length),
    ).toBe(1);
    expect(
      await page.textContent(".field-list li:not(.field-error) .field-value"),
    ).toBe("5");
  });

  test("computed values appear on the canvas card", async () => {
    const { page } = session;
    await page.click("button:text-is('Canvas')");
    // The list-mode note is unplaced, so it is not on the canvas.
    expect(await page.$$eval(".canvas-note", (els) => els.length)).toBe(0);
    const box = await page.locator(".canvas-view").boundingBox();
    await page.mouse.dblclick(box.x + 200, box.y + 150);
    await page.click("button:text-is('List')");
    await page.waitForFunction(
      () => document.querySelector(".note-title-input")?.value === "New Note",
    );
    await page.fill(".note-content-input", "alpha beta");
    await page.fill(".field-name-input", "n");
    await page.fill(".formula-source", "(count (words (content self)))");
    await page.click("button:text-is('Add field')");
    await page.click("button:text-is('Canvas')");
    await eventually(
      () =>
        page.$$eval(".canvas-note-fields .field-value", (els) =>
          els.map((e) => e.textContent),
        ),
      (vs) => expect(vs).toEqual(["2"]),
    );
  });

  test("no runtime errors", () => {
    expect(session.errors).toEqual([]);
  });
});

describe("persistence", () => {
  test("notes, links, positions and computed fields survive closing the browser", async () => {
    const profile = freshProfile();
    const first = await open(url, profile);
    const { page } = first;
    await newNote(page);
    await page.fill(".note-title-input", "Target");
    await newNote(page);
    await page.fill(".note-title-input", "Source");
    await page.fill(".note-content-input", "points at [[Target]]");
    await page.fill(".field-name-input", "links");
    await page.fill(".formula-source", "(count (links self))");
    await page.click("button:text-is('Add field')");
    await page.click("button:text-is('Canvas')");
    const box = await page.locator(".canvas-view").boundingBox();
    await page.mouse.dblclick(box.x + 420, box.y + 260);
    await eventually(
      () => cards(page),
      (cs) => expect(cs.length).toBe(1),
    );
    await autosaved(page);
    await first.context.close();

    const second = await open(url, profile);
    const p2 = second.page;
    expect(await sidebarTitles(p2)).toEqual(["New Note", "Source", "Target"]);
    await p2.click(".note-item[aria-label='Source']");
    expect(await p2.inputValue(".note-content-input")).toBe(
      "points at [[Target]]",
    );
    expect(await p2.textContent(".note-links .link-target")).toBe("Target");
    expect(await p2.textContent(".field-list .field-value")).toBe("1");
    await p2.click("button:text-is('Canvas')");
    const [card] = await eventually(
      () => cards(p2),
      (cs) => expect(cs.length).toBe(1),
    );
    near(card.left, 420);
    near(card.top, 260);
    expect(second.errors).toEqual([]);
    await second.context.close();
  });

  test("a corrupt autosave is quarantined, reported, and the app still starts", async () => {
    const profile = freshProfile();
    const first = await open(url, profile);
    await first.page.evaluate(
      () =>
        new Promise((resolve, reject) => {
          const req = indexedDB.open("nexia", 1);
          req.onupgradeneeded = () => req.result.createObjectStore("kv");
          req.onsuccess = () => {
            const tx = req.result.transaction("kv", "readwrite");
            tx.objectStore("kv").put('{"notes": {"broken', "nexia.autosave");
            tx.oncomplete = () => resolve();
            tx.onerror = () => reject(tx.error);
          };
        }),
    );
    await first.context.close();

    const second = await open(url, profile);
    const notice = await second.page.textContent(".notice-banner");
    expect(notice).toContain("could not be read");
    expect(notice).toContain("nexia.autosave.corrupt-");
    const keys = await second.page.evaluate(
      () =>
        new Promise((resolve) => {
          const req = indexedDB.open("nexia", 1);
          req.onsuccess = () => {
            const r = req.result
              .transaction("kv")
              .objectStore("kv")
              .getAllKeys();
            r.onsuccess = () => resolve(r.result);
          };
        }),
    );
    expect(
      keys.some((k) => String(k).startsWith("nexia.autosave.corrupt-")),
    ).toBe(true);
    // The app is usable on a fresh notebook.
    await newNote(second.page);
    expect(await sidebarTitles(second.page)).toEqual(["New Note"]);
    await second.context.close();
  });
});

describe("keyboard", () => {
  test("arrow keys move the selection spatially; Shift+arrow nudges; Delete removes", async () => {
    const { context, page, errors } = await open(url, freshProfile());
    await page.click("button:text-is('Canvas')");
    const box = await page.locator(".canvas-view").boundingBox();
    await page.mouse.dblclick(box.x + 100, box.y + 100);
    await page.keyboard.press("Escape");
    await page.mouse.dblclick(box.x + 400, box.y + 100);
    await page.keyboard.press("Escape");
    await eventually(
      () => cards(page),
      (cs) => expect(cs.length).toBe(2),
    );
    const selected = () =>
      page.$$eval(".canvas-note.selected", (els) =>
        els.map((e) => Math.round(parseFloat(e.style.left))),
      );
    expect(await selected()).toEqual([400]);
    await page.keyboard.press("ArrowLeft");
    await eventually(selected, (s) => expect(s).toEqual([100]));
    const top0 = await page.$eval(".canvas-note.selected", (e) =>
      parseFloat(e.style.top),
    );
    await page.keyboard.press("Shift+ArrowDown");
    await eventually(
      () => page.$eval(".canvas-note.selected", (e) => parseFloat(e.style.top)),
      (top) => near(top, top0 + 20),
    );
    await page.keyboard.press("Delete");
    await eventually(
      () => cards(page),
      (cs) => expect(cs.length).toBe(1),
    );
    expect(errors).toEqual([]);
    await context.close();
  });
});

describe("deep links", () => {
  test("views and the open note live in the URL; Back returns to the previous view", async () => {
    const profile = freshProfile();
    const { context, page, errors } = await open(url, profile);
    await newNote(page);
    await page.fill(".note-title-input", "Linked");
    await eventually(
      () =>
        page.$eval(".note-item.selected", (b) => b.getAttribute("aria-label")),
      (label) => expect(label).toBe("Linked"),
    );
    const hash = () => page.evaluate(() => location.hash);
    await eventually(hash, (h) =>
      expect(h).toMatch(/^#\/note\/[0-9a-f-]{36}$/),
    );
    const noteHash = await hash();
    const historyLength = await page.evaluate(() => history.length);

    await page.click("button:text-is('Canvas')");
    await eventually(hash, (h) => expect(h).toBe("#/canvas"));
    await page.click("button:text-is('Graph')");
    await eventually(hash, (h) => expect(h).toBe("#/graph"));
    // View changes add history entries; Back returns through them.
    expect(await page.evaluate(() => history.length)).toBe(historyLength + 2);
    await page.goBack();
    await page.waitForSelector(".canvas-view");
    await page.goBack();
    await page.waitForSelector(".note-editor");
    expect(await page.inputValue(".note-title-input")).toBe("Linked");
    await autosaved(page);
    await context.close();

    // A deep link opens that note directly on a fresh load.
    const again = await open(`${url}${noteHash}`, profile);
    await again.page.waitForSelector(".note-editor");
    expect(await again.page.inputValue(".note-title-input")).toBe("Linked");
    expect(errors).toEqual([]);
    expect(again.errors).toEqual([]);
    await again.context.close();
  });
});
