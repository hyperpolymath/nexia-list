// SPDX-License-Identifier: MPL-2.0
/// Dev server: rescript -w + esbuild serve with rebuild on request.
/// Run via: deno task dev

import * as esbuild from "esbuild";

const root = new URL("..", import.meta.url).pathname;

// ReScript watcher (compiles .res -> .res.js in-source)
const rescript = new Deno.Command(Deno.execPath(), {
  args: ["run", "-A", "npm:rescript@11.1.4", "build", "-w"],
  cwd: `${root}ui`,
  stdout: "inherit",
  stderr: "inherit",
}).spawn();

const ctx = await esbuild.context({
  entryPoints: [`${root}ui/src/Main.res.js`],
  bundle: true,
  format: "esm",
  sourcemap: true,
  outfile: `${root}web/dist/app.js`,
  loader: { ".wasm": "file" },
  logLevel: "info",
});

await ctx.watch();

// Copy static assets once; edit them with the server running and reload.
await Deno.mkdir(`${root}web/dist`, { recursive: true });
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
    await Deno.copyFile(`${root}web/${asset}`, `${root}web/dist/${asset}`);
  } catch (err) {
    if (!(err instanceof Deno.errors.NotFound)) throw err;
  }
}
try {
  await Deno.mkdir(`${root}web/dist/wasm`, { recursive: true });
  for await (const entry of Deno.readDir(`${root}web/wasm`)) {
    if (entry.isFile) {
      await Deno.copyFile(
        `${root}web/wasm/${entry.name}`,
        `${root}web/dist/wasm/${entry.name}`,
      );
    }
  }
} catch (err) {
  if (!(err instanceof Deno.errors.NotFound)) throw err;
}

const { hosts, port } = await ctx.serve({
  servedir: `${root}web/dist`,
  port: 5173,
});
console.log(`Nexia-List dev server: http://${hosts[0] ?? "localhost"}:${port}/`);

// Keep the process alive until interrupted; clean up the watcher on exit.
Deno.addSignalListener("SIGINT", () => {
  rescript.kill();
  esbuild.stop();
  Deno.exit(0);
});
await rescript.status;
