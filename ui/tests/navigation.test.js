// SPDX-License-Identifier: MPL-2.0
// Runs the pure keyboard-navigation geometry tests (NavigationTests.res).

import { runAll } from "./NavigationTests.res.js";
import { test } from "bun:test";

test("keyboard canvas navigation geometry", () => {
  runAll();
});
