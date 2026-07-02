// SPDX-License-Identifier: MPL-2.0
// Deno test wrapper: initializes the wasm core, then runs the ReScript TEA
// update tests (UpdateTests.res). Requires `deno task build:res` and
// `deno task build:wasm` first.

import initWasm from "../../web/wasm/nexia_core.js";
import { runAll } from "./UpdateTests.res.js";

const wasmBytes = await Deno.readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

Deno.test("TEA update delegates to the wasm core", () => {
  runAll();
});
