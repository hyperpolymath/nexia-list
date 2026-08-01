// SPDX-License-Identifier: MPL-2.0
/// Bun-native development server: ReScript watch + browser bundle rebuilds.
/// Run via: bun run dev

import { spawn } from "node:child_process";
import { copyFile, mkdir, readdir, watch } from "node:fs/promises";
import { extname, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const dist = `${root}web/dist`;

async function bundle() {
  const result = await Bun.build({
    entryPoints: [`${root}ui/src/Main.res.js`],
    target: "browser",
    format: "esm",
    sourcemap: "linked",
    outdir: dist,
    naming: "app.js",
  });
  if (!result.success) {
    for (const log of result.logs) console.error(log);
  }
}

async function copyAssets() {
  await mkdir(dist, { recursive: true });
  for (const asset of [
    "index.html",
    "styles.css",
    "manifest.webmanifest",
    "service-worker.js",
    "icon.svg",
  ]) {
    await copyFile(`${root}web/${asset}`, `${dist}/${asset}`);
  }
  try {
    await mkdir(`${dist}/wasm`, { recursive: true });
    for (const entry of await readdir(`${root}web/wasm`, {
      withFileTypes: true,
    })) {
      if (entry.isFile()) {
        await copyFile(
          `${root}web/wasm/${entry.name}`,
          `${dist}/wasm/${entry.name}`,
        );
      }
    }
  } catch (err) {
    if (err?.code !== "ENOENT") throw err;
  }
}

await copyAssets();
await bundle();

let rebuildTimer;
const watchAbort = new AbortController();
const watcher = watch(`${root}ui/src`, { recursive: true, signal: watchAbort.signal });
void (async () => {
  for await (const _event of watcher) {
    clearTimeout(rebuildTimer);
    rebuildTimer = setTimeout(() => void bundle(), 75);
  }
})();

const contentTypes = {
  ".css": "text/css",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".json": "application/json",
  ".map": "application/json",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".webmanifest": "application/manifest+json",
};

const server = Bun.serve({
  port: Number(process.env.PORT ?? 5173),
  async fetch(request) {
    const pathname = decodeURIComponent(new URL(request.url).pathname);
    const relative = pathname === "/" ? "index.html" : pathname.slice(1);
    const safe = normalize(relative).replace(/^\.\.(?:\/|\\|$)/, "");
    const file = Bun.file(`${dist}/${safe}`);
    if (!(await file.exists()))
      return new Response("Not found", { status: 404 });
    return new Response(file, {
      headers: {
        "content-type":
          contentTypes[extname(safe)] ?? "application/octet-stream",
      },
    });
  },
});

const rescript = spawn("bunx", ["rescript", "build", "-w"], {
  cwd: `${root}ui`,
  stdio: "inherit",
});

console.log(`Nexia-List dev server: ${server.url}`);

process.on("SIGINT", () => {
  clearTimeout(rebuildTimer);
  watchAbort.abort();
  rescript.kill();
  server.stop();
  process.exit(0);
});
