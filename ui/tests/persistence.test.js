// SPDX-License-Identifier: MPL-2.0
// Contract test for the persistence boundary through the wasm bindings: the
// exact path IndexedDB autosave takes (to_json -> stored string -> from_json),
// load reports for migrated/damaged data, computed fields, and rejection of
// geometry that JSON cannot store.

import initWasm, { WasmNotebook } from "../../web/wasm/nexia_core.js";
import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

const wasmBytes = await readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

const golden = await readFile(
  new URL("../../tests/fixtures/notebook.golden.json", import.meta.url),
  "utf8",
);

/** Build a small notebook exercising every persisted feature. */
function sample() {
  const nb = new WasmNotebook("Fidelity ✓");
  const a = nb.create_note_at("Alpha — αβγ", 12.5, -3.25);
  const b = nb.create_note("Beta");
  nb.update_content(a.id, "one two three [[Beta]] 🌱");
  nb.resize_note(b.id, 320, 0.1 + 0.2);
  nb.set_attribute(a.id, "status", JSON.stringify({ k: [1, 2.5, null] }));
  nb.setComputedField(a.id, "words", "(count (words (content self)))");
  nb.add_agent("todo", "status");
  return { nb, a, b };
}

test("to_json -> from_json is lossless and byte-stable", () => {
  const { nb, a, b } = sample();
  const saved = nb.to_json();
  const reloaded = WasmNotebook.from_json(saved);

  expect(reloaded.to_json()).toBe(saved);
  expect(reloaded.snapshot()).toEqual(nb.snapshot());
  expect(reloaded.backlinks(b.id)).toEqual([a.id]);
  expect(reloaded.get_note(b.id).size).toEqual([320, 0.1 + 0.2]);

  const report = reloaded.loadReport();
  expect(report.migrated).toBe(false);
  expect(report.danglingLinks).toEqual([]);
  expect(new WasmNotebook("fresh").loadReport()).toBeNull();
});

test("legacy (v1) data migrates; newer schema is refused", () => {
  const nb = WasmNotebook.from_json(golden);
  const report = nb.loadReport();
  expect(report.fromVersion).toBe(1);
  expect(report.migrated).toBe(true);
  expect(JSON.parse(nb.to_json()).schema_version).toBe(2);

  const future = JSON.parse(golden);
  future.schema_version = 99;
  expect(() => WasmNotebook.from_json(JSON.stringify(future))).toThrow(
    /newer than this build supports/,
  );
});

test("corrupt stored data throws a JS Error the caller can quarantine", () => {
  for (const bad of ["", "{", "null", '{"notes": 1}', golden.slice(0, 40)]) {
    expect(() => WasmNotebook.from_json(bad)).toThrow(/corrupt notebook data/);
  }
});

test("damaged links are repaired and reported", () => {
  const doc = JSON.parse(golden);
  const [first] = Object.keys(doc.notes);
  const ghost = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
  doc.notes[first].links = [...(doc.notes[first].links ?? []), ghost, first];
  const nb = WasmNotebook.from_json(JSON.stringify(doc));
  const report = nb.loadReport();
  expect(report.danglingLinks).toEqual([[first, ghost]]);
  expect(report.selfLinks).toEqual([first]);
  expect(nb.get_note(first).links).not.toContain(ghost);
});

test("computed fields persist as source and evaluate after reload", () => {
  const { nb, a } = sample();
  nb.setComputedField(a.id, "bad", "(car)");
  const reloaded = WasmNotebook.from_json(nb.to_json());

  expect(reloaded.get_note(a.id).computed).toEqual({
    bad: "(car)",
    words: "(count (words (content self)))",
  });
  const fields = reloaded.evalComputedFields(a.id);
  expect(fields.map((f) => f.name)).toEqual(["bad", "words"]);
  expect(typeof fields[0].error).toBe("string");
  expect(fields[0].value).toBeUndefined();
  expect(fields[1]).toEqual({
    name: "words",
    source: "(count (words (content self)))",
    value: "5",
  });

  reloaded.removeComputedField(a.id, "bad");
  expect(Object.keys(reloaded.get_note(a.id).computed)).toEqual(["words"]);
  expect(() => reloaded.setComputedField(a.id, "  ", "1")).toThrow();
  expect(() =>
    reloaded.evalComputedFields("00000000-0000-4000-8000-000000000000"),
  ).toThrow(/Note not found/);
});

test("non-finite geometry is rejected before it can poison a save", () => {
  const { nb, a } = sample();
  for (const [x, y] of [
    [Number.NaN, 0],
    [0, Number.POSITIVE_INFINITY],
  ]) {
    expect(() => nb.move_note(a.id, x, y)).toThrow(/finite/);
    expect(() => nb.resize_note(a.id, x, y)).toThrow(/finite/);
    expect(() => nb.create_note_at("x", x, y)).toThrow(/finite/);
  }
  expect(nb.get_note(a.id).position).toEqual({ x: 12.5, y: -3.25 });
  expect(() => WasmNotebook.from_json(nb.to_json())).not.toThrow();
});
