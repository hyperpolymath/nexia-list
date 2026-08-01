// SPDX-License-Identifier: MPL-2.0
// Contract test for agents across the wasm boundary.

import initWasm, { WasmNotebook } from "../../web/wasm/nexia_core.js";
import { test } from "bun:test";
import { readFile } from "node:fs/promises";

const wasmBytes = await readFile(
  new URL("../../web/wasm/nexia_core_bg.wasm", import.meta.url),
);
await initWasm({ module_or_path: wasmBytes });

function assert(cond, label) {
  if (!cond) throw new Error(`agents contract violated: ${label}`);
}

test("agents collect matching notes and persist", () => {
  const nb = new WasmNotebook("Test");
  const todo = nb.create_note("Buy milk");
  nb.set_attribute(todo.id, "status", JSON.stringify("todo"));
  const done = nb.create_note("Ship it");
  nb.set_attribute(done.id, "status", JSON.stringify("done"));

  const agent = nb.add_agent("Open tasks", "attr:status=todo");
  assert(
    typeof agent.id === "string" && agent.name === "Open tasks",
    "agent view shape",
  );

  const agents = nb.agents();
  assert(Array.isArray(agents) && agents.length === 1, "one agent listed");

  const collected = nb.run_agent(agent.id);
  assert(
    collected.length === 1 && collected[0] === todo.id,
    "agent collects the todo note",
  );

  // Ad-hoc query preview.
  const preview = nb.run_query("title:ship");
  assert(
    preview.length === 1 && preview[0] === done.id,
    "run_query previews matches",
  );

  // Persist through the on-disk JSON and reload.
  const json = nb.to_json();
  const reloaded = WasmNotebook.from_json(json);
  assert(reloaded.agents().length === 1, "agent survives round-trip");
  assert(
    reloaded.run_agent(agent.id)[0] === todo.id,
    "reloaded agent still collects",
  );

  assert(nb.remove_agent(agent.id) === true, "remove returns true");
  assert(nb.agents().length === 0, "agent removed");
});
