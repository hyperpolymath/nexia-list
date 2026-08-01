// SPDX-License-Identifier: MPL-2.0
/// Bun production bundle: ui/src/Main.res.js -> web/dist/
/// Run via: bun run build:web  (after bun run build:res)

import { copyFile, mkdir, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const dist = `${root}web/dist`;

await mkdir(dist, { recursive: true });

const result = await Bun.build({
  entryPoints: [`${root}ui/src/Main.res.js`],
  target: "browser",
  format: "esm",
  minify: true,
  sourcemap: "linked",
  outdir: dist,
  naming: "app.js",
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}

// Static assets. index.html references ./app.js and ./styles.css.
for (const asset of [
  "index.html",
  "styles.css",
  "manifest.webmanifest",
  "service-worker.js",
  "icon.svg",
]) {
  try {
    await copyFile(`${root}web/${asset}`, `${dist}/${asset}`);
  } catch (err) {
    if (err?.code !== "ENOENT") throw err;
  }
}

// WASM core artifacts (present after bun run build:wasm).
try {
  await mkdir(`${dist}/wasm`, { recursive: true });
  for (const entry of await readdir(`${root}web/wasm`, {
    withFileTypes: true,
  })) {
    if (entry.isFile) {
      await copyFile(
        `${root}web/wasm/${entry.name}`,
        `${dist}/wasm/${entry.name}`,
      );
    }
  }
} catch (err) {
  if (err?.code !== "ENOENT") throw err;
}
