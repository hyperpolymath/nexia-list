// SPDX-License-Identifier: MPL-2.0
/// Build the Rust core to WebAssembly with JS glue in web/wasm/.
/// Run via: deno task build:wasm
/// Requires: rustup target wasm32-unknown-unknown, wasm-bindgen-cli
/// (cargo install wasm-bindgen-cli --version <matching Cargo.lock>).

const root = new URL("..", import.meta.url).pathname;

async function run(cmd, args, cwd) {
  const status = await new Deno.Command(cmd, {
    args,
    cwd,
    stdout: "inherit",
    stderr: "inherit",
  }).spawn().status;
  if (!status.success) {
    console.error(`${cmd} ${args.join(" ")} failed (exit ${status.code})`);
    Deno.exit(status.code);
  }
}

await run("cargo", [
  "build",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--features",
  "wasm",
  "-p",
  "nexia-core",
], root);

await run("wasm-bindgen", [
  `${root}target/wasm32-unknown-unknown/release/nexia_core.wasm`,
  "--target",
  "web",
  "--no-typescript",
  "--out-dir",
  `${root}web/wasm`,
], root);

console.log("WASM core built: web/wasm/nexia_core.js + nexia_core_bg.wasm");
