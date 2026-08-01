// SPDX-License-Identifier: MPL-2.0
/// Build the Rust core to WebAssembly with JS glue in web/wasm/.
/// Run via: bun run build:wasm
/// Requires: rustup target wasm32-unknown-unknown, wasm-bindgen-cli
/// (cargo install wasm-bindgen-cli --version <matching Cargo.lock>).

import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));

async function run(cmd, args, cwd) {
  const code = await new Promise((resolve, reject) => {
    const child = spawn(cmd, args, { cwd, stdio: "inherit" });
    child.on("error", reject);
    child.on("close", resolve);
  });
  if (code !== 0)
    throw new Error(`${cmd} ${args.join(" ")} failed (exit ${code})`);
}

await run(
  "cargo",
  [
    "build",
    "--release",
    "--target",
    "wasm32-unknown-unknown",
    "--features",
    "wasm",
    "-p",
    "nexia-core",
  ],
  root,
);

await run(
  "wasm-bindgen",
  [
    `${root}target/wasm32-unknown-unknown/release/nexia_core.wasm`,
    "--target",
    "web",
    "--no-typescript",
    "--out-dir",
    `${root}web/wasm`,
  ],
  root,
);

console.log("WASM core built: web/wasm/nexia_core.js + nexia_core_bg.wasm");
