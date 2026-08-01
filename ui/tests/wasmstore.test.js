// SPDX-License-Identifier: MPL-2.0
// Exercises the ReScript WasmStore facade, rather than calling wasm-bindgen's
// generated JavaScript API directly.

import initWasm from "../../web/wasm/nexia_core.js";
import { runAll } from "./WasmStoreTests.res.js";
import { test } from "bun:test";
import { readFile } from "node:fs/promises";

const wasmBytes = await readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

test("WasmStore binds the complete Rust/WASM API", () => {
  runAll();
});
