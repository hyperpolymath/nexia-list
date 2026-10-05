// SPDX-License-Identifier: MPL-2.0
// Contract test for the paged search binding the UI uses (searchPage).

import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import initWasm, { WasmNotebook } from "../../web/wasm/nexia_core.js";

await initWasm({
  module_or_path: await readFile(
    new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
  ),
});

test("searchPage returns the first `limit` matches by title and the total", () => {
  const nb = new WasmNotebook("s");
  for (const t of [
    "delta garden",
    "alpha garden",
    "charlie",
    "bravo garden",
    "echo GARDEN",
  ]) {
    nb.create_note(t);
  }
  const page = nb.searchPage("garden", 2);
  expect(page.total).toBe(4);
  expect(page.ids.map((id) => nb.get_note(id).title)).toEqual([
    "alpha garden",
    "bravo garden",
  ]);
  const all = nb.searchPage("garden", 100);
  expect(all.ids.map((id) => nb.get_note(id).title)).toEqual([
    "alpha garden",
    "bravo garden",
    "delta garden",
    "echo GARDEN",
  ]);
  expect(nb.searchPage("", 10)).toEqual({ ids: [], total: 0 });
  expect(nb.searchPage("zzz", 10)).toEqual({ ids: [], total: 0 });
});

test("search results follow edits (the core's search cache is invalidated)", () => {
  const nb = new WasmNotebook("s");
  const n = nb.create_note("one");
  expect(nb.searchPage("one", 10).total).toBe(1);
  nb.update_title(n.id, "two");
  expect(nb.searchPage("one", 10).total).toBe(0);
  expect(nb.searchPage("two", 10).total).toBe(1);
});
