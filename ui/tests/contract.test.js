// SPDX-License-Identifier: MPL-2.0
// Golden-fixture contract test — the JS half of core/tests/golden.rs.
// Both sides decode tests/fixtures/notebook.golden.json; a serde shape
// change that breaks the UI contract fails here.

import initWasm, {
  lambdadeltaEval,
  WasmNotebook,
} from "../../web/wasm/nexia_core.js";
import { test } from "bun:test";
import { readFile } from "node:fs/promises";

const wasmBytes = await readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

const fixture = await readFile(
  new URL("../../tests/fixtures/notebook.golden.json", import.meta.url),
  "utf8",
);
const lambdadeltaVectors = JSON.parse(
  await readFile(
    new URL(
      "../../tests/fixtures/lambdadelta-conformance.json",
      import.meta.url,
    ),
    "utf8",
  ),
);

const ALPHA = "11111111-1111-4111-8111-111111111111";
const BETA = "22222222-2222-4222-8222-222222222222";

function assert(cond, label) {
  if (!cond) throw new Error(`contract violated: ${label}`);
}

test("golden fixture decodes to the UI shape", () => {
  const nb = WasmNotebook.from_json(fixture);
  const snap = nb.snapshot();

  assert(snap.name === "Golden", "notebook name");
  assert(Object.keys(snap.notes).length === 2, "two notes");

  const alpha = snap.notes[ALPHA];
  assert(alpha.title === "Alpha", "title");
  assert(
    alpha.createdAt.startsWith("2026-01-01T00:00:00"),
    "camelCase createdAt",
  );
  assert(
    alpha.modifiedAt.startsWith("2026-01-02T00:00:00"),
    "camelCase modifiedAt",
  );
  assert(
    Array.isArray(alpha.links) && alpha.links.includes(BETA),
    "links present",
  );
  assert(
    alpha.position && alpha.position.x === 10 && alpha.position.y === 20,
    "position object",
  );
  assert(Array.isArray(alpha.size) && alpha.size[0] === 200, "size tuple");
  assert(
    alpha.attributes && alpha.attributes.status === "todo",
    "attributes object",
  );

  const beta = snap.notes[BETA];
  assert(
    Array.isArray(beta.links) && beta.links.length === 0,
    "empty links always present",
  );
  assert(
    beta.position === undefined,
    "absent position is undefined (ReScript None)",
  );

  // Backlinks rebuilt even though the fixture omits them.
  assert(snap.backlinks[BETA].includes(ALPHA), "backlinks rebuilt on load");
});

test("mutations return granular views; disk format stays snake_case", () => {
  const nb = WasmNotebook.from_json(fixture);

  const created = nb.create_note("Gamma");
  assert(
    typeof created.id === "string" && created.id.length === 36,
    "core-issued UUID",
  );
  assert(created.links.length === 0, "fresh note has no links");

  const delta = nb.link(created.id, ALPHA);
  assert(delta.changed[0].links.includes(ALPHA), "delta carries changed note");
  assert(
    delta.backlinks[ALPHA].includes(created.id),
    "delta carries backlink entry",
  );

  const ids = nb.search("gamma");
  assert(ids.length === 1 && ids[0] === created.id, "search finds new note");

  const disk = JSON.parse(nb.to_json());
  assert(
    "created_at" in Object.values(disk.notes)[0],
    "disk format is snake_case",
  );
  assert(disk.name === "Golden", "disk name preserved");
});

test("WASM λδ kernel matches the native conformance vectors", () => {
  for (const vector of lambdadeltaVectors) {
    const actual = lambdadeltaEval(vector.source);
    assert(actual === vector.printed, `λδ vector: ${vector.name}`);
  }
});
