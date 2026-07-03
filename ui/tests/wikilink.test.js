// SPDX-License-Identifier: MPL-2.0
// Contract: update_content derives [[Title]] links and returns a delta.

import initWasm, { WasmNotebook } from "../../web/wasm/nexia_core.js";

const wasmBytes = await Deno.readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

function assert(cond, label) {
  if (!cond) throw new Error(`wikilink contract violated: ${label}`);
}

Deno.test("editing content derives wiki-links and returns a delta", () => {
  const nb = new WasmNotebook("Test");
  const alpha = nb.create_note("Alpha");
  const beta = nb.create_note("Beta");

  const delta = nb.update_content(alpha.id, "see [[Beta]] here");
  // The edited note is in `changed` with the new link.
  assert(delta.changed.length === 1, "one changed note");
  assert(delta.changed[0].links.includes(beta.id), "derived link to Beta");
  // Beta's backlinks now include Alpha.
  assert(delta.backlinks[beta.id].includes(alpha.id), "delta carries Beta backlink");

  // The link persists in the notebook snapshot.
  const snap = nb.snapshot();
  assert(snap.notes[alpha.id].links.includes(beta.id), "link persisted");
  assert(snap.backlinks[beta.id].includes(alpha.id), "backlink persisted");

  // Editing again without a resolvable target adds nothing.
  const delta2 = nb.update_content(alpha.id, "still [[Beta]] and [[Ghost]]");
  assert(Object.keys(delta2.backlinks).length <= 1, "no new backlink for unresolved Ghost");
  assert(snap.notes[alpha.id].links.length === 1, "no duplicate link");
});
