// SPDX-License-Identifier: MPL-2.0
// Contract test for the import/export boundary through the wasm bindings.

import initWasm, { WasmNotebook } from "../../web/wasm/nexia_core.js";

const wasmBytes = await Deno.readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

function assert(cond, label) {
  if (!cond) throw new Error(`exchange contract violated: ${label}`);
}

Deno.test("markdown export shape and OPML", () => {
  const nb = new WasmNotebook("Test");
  const alpha = nb.create_note("Alpha");
  const beta = nb.create_note("Beta");
  nb.link(alpha.id, beta.id);

  const files = nb.export_markdown();
  assert(Array.isArray(files) && files.length === 2, "two markdown files");
  const alphaFile = files.find((f) => f.name === "Alpha.md");
  assert(alphaFile, "Alpha.md present");
  assert(alphaFile.content.includes("# Alpha"), "H1 title");
  assert(alphaFile.content.includes("[[Beta]]"), "wiki-link to Beta");

  const opml = nb.export_opml();
  assert(opml.includes('<opml version="2.0">'), "opml root");
  assert(opml.includes('text="Alpha"'), "opml outline for Alpha");
});

Deno.test("markdown round-trip preserves titles and links", () => {
  const nb = new WasmNotebook("Test");
  const a = nb.create_note("Alpha");
  const b = nb.create_note("Beta");
  nb.link(a.id, b.id);
  const files = nb.export_markdown();

  const fresh = new WasmNotebook("Empty");
  const snap = fresh.import_markdown_vault(files);

  assert(Object.keys(snap.notes).length === 2, "two notes imported");
  const titles = Object.values(snap.notes).map((n) => n.title).sort();
  assert(titles[0] === "Alpha" && titles[1] === "Beta", "titles preserved");

  const alpha = Object.values(snap.notes).find((n) => n.title === "Alpha");
  const beta = Object.values(snap.notes).find((n) => n.title === "Beta");
  assert(alpha.links.includes(beta.id), "forward link preserved");
  assert(snap.backlinks[beta.id].includes(alpha.id), "backlink rebuilt");
});

Deno.test("import creates placeholders for unresolved wiki-links", () => {
  const fresh = new WasmNotebook("Empty");
  const snap = fresh.import_markdown_vault([
    { name: "One.md", content: "# One\n\nsee [[Ghost]]\n" },
  ]);
  const titles = Object.values(snap.notes).map((n) => n.title).sort();
  assert(titles.includes("Ghost"), "placeholder note created");
  assert(Object.keys(snap.notes).length === 2, "two notes total");
});
