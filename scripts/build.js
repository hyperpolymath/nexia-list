// SPDX-License-Identifier: MPL-2.0
/// Production bundle: ui/src/Main.res.js -> web/dist/
/// Run via: deno task build:web  (after deno task build:res)

import * as esbuild from "esbuild";

const root = new URL("..", import.meta.url).pathname;
const dist = `${root}web/dist`;

await Deno.mkdir(dist, { recursive: true });

await esbuild.build({
  entryPoints: [`${root}ui/src/Main.res.js`],
  bundle: true,
  format: "esm",
  minify: true,
  sourcemap: true,
  outfile: `${dist}/app.js`,
  loader: { ".wasm": "file" },
  logLevel: "info",
});

// Static assets. index.html references ./app.js and ./styles.css.
for (
  const asset of [
    "index.html",
    "styles.css",
    "manifest.webmanifest",
    "service-worker.js",
    "icon.svg",
  ]
) {
  try {
    await Deno.copyFile(`${root}web/${asset}`, `${dist}/${asset}`);
  } catch (err) {
    if (!(err instanceof Deno.errors.NotFound)) throw err;
  }
}

// WASM core artifacts (present after deno task build:wasm).
try {
  await Deno.mkdir(`${dist}/wasm`, { recursive: true });
  for await (const entry of Deno.readDir(`${root}web/wasm`)) {
    if (entry.isFile) {
      await Deno.copyFile(
        `${root}web/wasm/${entry.name}`,
        `${dist}/wasm/${entry.name}`,
      );
    }
  }
} catch (err) {
  if (!(err instanceof Deno.errors.NotFound)) throw err;
}

esbuild.stop();
