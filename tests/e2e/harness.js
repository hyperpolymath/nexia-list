// SPDX-License-Identifier: MPL-2.0
// Shared harness for the Nexia-List browser tests: serve web/dist, launch
// Chromium (optionally with a persistent profile so "close the browser and
// reopen it" is real), and helpers to read the app's state from the DOM.

import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright";

export const dist = new URL("../../web/dist/", import.meta.url).pathname;

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".webmanifest": "application/manifest+json",
};

/** Serve web/dist on a free port; throws if it has not been built. */
export function serve() {
  if (!existsSync(join(dist, "app.bun.js"))) {
    throw new Error(
      "web/dist is not built: run `bun run build` at the repo root first",
    );
  }
  return Bun.serve({
    port: 0,
    fetch(req) {
      const path = new URL(req.url).pathname;
      const file = join(dist, path === "/" ? "index.html" : path.slice(1));
      const ext = file.slice(file.lastIndexOf("."));
      return new Response(Bun.file(file), {
        headers: { "content-type": TYPES[ext] ?? "application/octet-stream" },
      });
    },
  });
}

/** A fresh profile directory (IndexedDB lives here across relaunches). */
export const freshProfile = () =>
  mkdtempSync(join(tmpdir(), "nexia-e2e-profile-"));

/**
 * Launch Chromium on `profile` and open the app. Returns { context, page,
 * errors } where `errors` collects console errors and uncaught exceptions.
 */
export async function open(
  url,
  profile,
  viewport = { width: 1280, height: 800 },
) {
  const context = await chromium.launchPersistentContext(profile, {
    viewport,
    serviceWorkers: "block",
  });
  const page = context.pages()[0] ?? (await context.newPage());
  const errors = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(url);
  await page.waitForSelector(".toolbar");
  return { context, page, errors };
}

/**
 * Poll `read` until `check(value)` passes (or `ms` elapse), then assert it.
 * Renders are batched to the next animation frame, so reads after an
 * interaction must wait for it.
 */
export async function eventually(read, check, ms = 5000) {
  const deadline = Date.now() + ms;
  let value = await read();
  while (Date.now() < deadline) {
    try {
      check(value);
      return value;
    } catch {
      await new Promise((r) => setTimeout(r, 20));
      value = await read();
    }
  }
  check(value);
  return value;
}

/** Wait until the autosave has written the current revision to IndexedDB. */
export async function autosaved(page) {
  await page.waitForFunction(() => {
    const rev = document.querySelector(".app")?.dataset.rev;
    return (
      rev !== undefined && document.documentElement.dataset.autosaved === rev
    );
  });
}
