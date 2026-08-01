// SPDX-License-Identifier: MPL-2.0
import { runAll } from "./GraphLayoutTests.res.js";
import { test } from "bun:test";

test("graph layout geometry", () => {
  runAll();
});
