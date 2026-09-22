#!/usr/bin/env node
// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// ld-mint — the λδ package minter (issue #33). Scaffolds a new plugin:
// manifest, entry point, harness test, and docs stub, so authoring starts
// from a working template rather than a blank file.
//
// Runtime-agnostic (works on Bun and Node; the estate toolchain invokes it via
// Bun):   bun scripts/ld-mint.js <name> [--tier ayo|teranga|shield]
//           [--caps notes/read,notes/write] [--description "…"] [--dir plugins]
//
// The minted shape mirrors boj-server-cartridges (cartridge.json fields map
// 1:1 onto manifest.ld keywords) and the PanLL minter contract
// (panll/contracts/minter.toml) — see docs/design/lambdadelta-plugin-system.adoc.

import fs from "node:fs";
import path from "node:path";

const VALID_TIERS = new Set(["teranga", "shield", "ayo"]);
const VALID_CAPS = new Set(["notes/read", "notes/write", "agents/run"]);

function die(msg) {
  console.error(`ld-mint: ${msg}`);
  process.exit(2);
}

function parseArgs(argv) {
  const args = {
    name: null,
    tier: "ayo",
    caps: ["notes/read"],
    description: null,
    dir: "plugins",
  };
  const rest = [...argv];
  while (rest.length > 0) {
    const a = rest.shift();
    if (!a.startsWith("--")) {
      if (args.name !== null) die(`unexpected extra argument: ${a}`);
      args.name = a;
      continue;
    }
    const val = rest.shift();
    if (val === undefined) die(`flag ${a} needs a value`);
    switch (a) {
      case "--tier":
        if (!VALID_TIERS.has(val)) die(`unknown tier ${val} (teranga|shield|ayo)`);
        args.tier = val;
        break;
      case "--caps": {
        const caps = val.split(",").map((c) => c.trim().replace(/^:/, ""));
        for (const c of caps) {
          if (!VALID_CAPS.has(c)) die(`unknown capability ${c} (${[...VALID_CAPS].join(", ")})`);
        }
        args.caps = caps;
        break;
      }
      case "--description":
        args.description = val;
        break;
      case "--dir":
        args.dir = val;
        break;
      default:
        die(`unknown flag ${a}`);
    }
  }
  return args;
}

function kebabCase(name) {
  return (
    name.length > 0 &&
    /^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)
  );
}

const MANIFEST = (a) => `; SPDX-License-Identifier: MPL-2.0
; manifest.ld — λδ package manifest for '${a.name}'. Homoiconic: this is a λδ
; map literal, validated by nexia-core (lambdadelta::package::PackageManifest).
{
 :name "${a.name}"
 :version "0.1.0"
 :spdx "MPL-2.0"
 :tier :${a.tier}                       ; :teranga core | :shield elevated-trust | :ayo community
 :description "${a.description ?? `${a.name} — a λδ package for Nexia-List`}"
 :entry-point "src/main.ld"
 :capabilities [${a.caps.map((c) => `:${c}`).join(" ")}]
 ;; Configurator schema: the settings UI is generated from this map; no value
 ;; reaches the plugin unvalidated (PackageManifest::resolve_config).
 ;:config {:example {:type :int :default 0 :doc "an example setting"}}
 :tests ["test/main.test.ld"]
}
`;

const MAIN_LD = (a) => `;; SPDX-License-Identifier: MPL-2.0
;; ${a.name}/src/main.ld — package entry point (loaded by the provisioned
;; interpreter; definitions persist for the host to call).
;;
;; Available builtins depend on the grants in manifest.ld :capabilities —
;; nothing runs with capabilities the user hasn't granted:
;;   :notes/read  → (notes) (note id) (attr n :key) (search s) …
;;   :notes/write → (create-note! title) (set-attr! id "key" v) …
;;   :agents/run  → (run-agent "name")

;; Example: a pure helper over the notebook surface.
(def note-titles
  (fn [] (map (fn [n] (:title n)) (notes))))
`;

const TEST_LD = (a) => `;; SPDX-License-Identifier: MPL-2.0
;; ${a.name}/test/main.test.ld — harness tests. assert-eq/assert RECORD into a
;; report (they never abort), and an evaluation error in this file becomes a
;; failed assertion — the harness report is the whole story.
;;
;; Run locally via the Rust integration tests (core/tests/) which build the
;; fixture notebook this package expects, or in CI the same way.

(assert-eq 0 0)  ; replace with real assertions, e.g.:
;; (assert-eq 3 (count (notes)))
`;

const README = (a) => `// SPDX-License-Identifier: CC-BY-SA-4.0
= ${a.name}
:tier: ${a.tier}

${a.description ?? `A λδ package for Nexia-List.`}

Minted with \`just ld-new ${a.name}\` (scripts/ld-mint.js — the λδ minter, issue #33).

== Layout

[cols="1,2"]
|===
| \`manifest.ld\`        | Homoiconic manifest: version, tier (_{a.tier}_), requested capabilities, config schema
| \`src/main.ld\`        | Entry point (definitions loaded by the provisioned interpreter)
| \`test/main.test.ld\`  | Harness assertions (\`assert-eq\`/\`assert\`), run in the sandbox with the enforced grants
| \`README.adoc\`        | This file
|===

== Lifecycle

. *mint* — you are here
. *develop* — edit \`src/main.ld\`; iterate with the harness (\`assert-eq\` in \`test/\`)
. *provision* — the provisioner checks the requested capabilities against user grants
. *configure* — user settings validated against the manifest's \`:config\` schema
. *run* — inside the sandbox: enforced capabilities + evaluation budget
`;

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.name === null) {
    console.error(`usage: ld-mint <name> [--tier ayo|teranga|shield] [--caps notes/read,notes/write] [--description "…"] [--dir plugins]`);
    process.exit(2);
  }
  if (!kebabCase(args.name)) {
    die(`invalid name ${args.name} — kebab-case (a-z, 0-9, single dashes)`);
  }
  const root = path.join(args.dir, args.name);
  if (fs.existsSync(root)) {
    die(`${root} already exists — refusing to overwrite`);
  }

  const files = {
    "manifest.ld": MANIFEST(args),
    "src/main.ld": MAIN_LD(args),
    "test/main.test.ld": TEST_LD(args),
    "README.adoc": README(args),
  };
  for (const [rel, contents] of Object.entries(files)) {
    const p = path.join(root, rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, contents);
    console.log(`  minted ${p}`);
  }
  console.log(`\nλδ package '${args.name}' minted (tier ${args.tier}).`);
  console.log(`Next: edit ${path.join(root, "src/main.ld")}, then prove it in the harness.`);
}

main();
