// SPDX-License-Identifier: MPL-2.0
// Unit tests for ui/src/Geometry.affine (pure canvas geometry), compiled with
// the AffineScript compiler's Bun-ESM backend and called directly.

import { beforeAll, expect, test } from "bun:test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = new URL("../../", import.meta.url).pathname;
const home = process.env.AFFINESCRIPT_HOME ?? join(root, ".affinescript");
let G;

beforeAll(async () => {
  const out = join(
    mkdtempSync(join(tmpdir(), "nexia-geom-")),
    "geometry.bun.js",
  );
  const r = Bun.spawnSync({
    cmd: [
      join(home, "_build/default/bin/main.exe"),
      "compile",
      "--bun-esm",
      "Geometry.affine",
      "-o",
      out,
    ],
    cwd: join(root, "ui/src"),
    env: { ...process.env, AFFINESCRIPT_STDLIB: join(home, "stdlib") },
  });
  if (r.exitCode !== 0) {
    throw new Error(
      `compiling Geometry.affine failed (AFFINESCRIPT_HOME=${home}; run scripts/fetch-affinescript.sh):\n${r.stdout}${r.stderr}`,
    );
  }
  G = await import(out);
});

/** A placed note at (x, y). */
const note = (id, x, y, placed = true) => ({
  id,
  title: id,
  content: "",
  placed,
  x,
  y,
  w: 0,
  h: 0,
  links: [],
  created: "",
  modified: "",
  field_count: 0,
});

test("nearest picks the closest note in the direction, penalising drift", () => {
  const a = note("a", 0, 0);
  const right = note("r", 100, 0);
  const farRight = note("fr", 300, 0);
  const diag = note("d", 60, 50);
  const notes = [a, right, farRight, diag];
  expect(G.nearest(notes, a, G.Right)).toBe("r");
  expect(G.nearest(notes, a, G.Down)).toBe("d");
  expect(G.nearest(notes, a, G.Left)).toBe("");
  expect(G.nearest(notes, note("u", 0, 0, false), G.Right)).toBe("");
});

test("nudge moves one step in the direction", () => {
  expect(G.nudge(10, 10, G.Up)).toEqual([10, -10]);
  expect(G.nudge(10, 10, G.Right)).toEqual([30, 10]);
});

test("zoom is clamped and panning accumulates", () => {
  const v = G.initial_viewport();
  expect(G.zoom_by(v, 100).zoom).toBe(5);
  expect(G.zoom_by(v, 0.001).zoom).toBe(0.1);
  expect(G.pan_by(G.pan_by(v, 10, 5), -4, 1)).toEqual({
    ox: 6,
    oy: 6,
    zoom: 1,
  });
});

test("screen points map to canvas coordinates", () => {
  const v = { ox: 100, oy: 50, zoom: 2 };
  expect(G.to_canvas(v, 300, 250)).toEqual([100, 100]);
});

test("visibility culls notes outside the viewport (with a margin)", () => {
  const v = G.initial_viewport();
  expect(G.visible(v, 800, 600, note("in", 100, 100))).toBe(true);
  expect(G.visible(v, 800, 600, note("edge", 820, 100))).toBe(true);
  expect(G.visible(v, 800, 600, note("out", 5000, 100))).toBe(false);
  expect(G.visible(v, 800, 600, note("unplaced", 100, 100, false))).toBe(false);
  expect(
    G.visible(
      { ox: -4900, oy: 0, zoom: 1 },
      800,
      600,
      note("panned", 5000, 100),
    ),
  ).toBe(true);
});

test("the circular layout is finite, inside the box, and starts at the top", () => {
  const nodes = G.circular(
    [note("a", 0, 0), note("b", 0, 0), note("c", 0, 0), note("d", 0, 0)],
    800,
    600,
  );
  expect(nodes.map((n) => n.id)).toEqual(["a", "b", "c", "d"]);
  for (const n of nodes) {
    expect(Number.isFinite(n.x) && Number.isFinite(n.y)).toBe(true);
    expect(n.x >= 0 && n.x <= 800 && n.y >= 0 && n.y <= 600).toBe(true);
  }
  expect(nodes[0].x).toBeCloseTo(400);
  expect(nodes[0].y).toBeLessThan(300);
  expect(G.circular([], 800, 600)).toEqual([]);
});
