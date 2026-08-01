<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-07-17 -->

# Nexia-List — Project Topology

## System Architecture

Nexia-List is **web-first**: the browser is the primary target, running the real
Rust core compiled to WebAssembly. The core carries the **λδ (LambdaDelta)**
substrate — a homoiconic Clojure-flavoured Lisp built in-tree, not vendored —
which is the single largest subsystem in the core. The desktop shell is optional
and depends on an **external** sibling checkout of `hyperpolymath/gossamer`; it
is intentionally not built in this repo's CI.

```
                        ┌─────────────────────────────────────────┐
                        │              USER INTERFACE             │
                        │     (Note list / Editor / Canvas)       │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           UI LAYER (RESCRIPT)           │
                        │  Hand-rolled TEA on @rescript/react     │
                        │  Model / Msg / Update / View            │
                        │  Bun bundle → web/dist/                 │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │        BROWSER (PRIMARY TARGET)         │
                        │                                         │
                        │  ┌───────────────────────────────────┐  │
                        │  │   RUST CORE → WASM (wasm-bindgen) │  │
                        │  │   Note · Notebook · Backlinks     │  │
                        │  │   Substring search · JSON storage │  │
                        │  │   Agents · Markdown/OPML exchange │  │
                        │  │                                   │  │
                        │  │  ┌─────────────────────────────┐  │  │
                        │  │  │  λδ (LAMBDADELTA) SUBSTRATE │  │  │
                        │  │  │  reader · value model       │  │  │
                        │  │  │  evaluator + Budget sandbox │  │  │
                        │  │  │  hygienic macros · prelude  │  │  │
                        │  │  │  multimethods on :type/:op  │  │  │
                        │  │  │  notebook host seam         │  │  │
                        │  │  │  (~64% of core LOC)         │  │  │
                        │  │  └─────────────────────────────┘  │  │
                        │  └────────────────┬──────────────────┘  │
                        │                   │                     │
                        │                   ▼                     │
                        │  ┌───────────────────────────────────┐  │
                        │  │          DATA LAYER               │  │
                        │  │  IndexedDB + file download/upload │  │
                        │  │  (human-readable JSON)            │  │
                        │  └───────────────────────────────────┘  │
                        └───────────────────┬─────────────────────┘
                                            │ optional
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │  GOSSAMER DESKTOP SHELL (OPTIONAL)      │
                        │  EXTERNAL: requires sibling checkout    │
                        │  ../gossamer — NOT built in this CI     │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Bun tasks      Justfile   scripts/     │
                        │  rust-ci.yml    ui-ci.yml               │
                        │  .machine_readable/  0-AI-MANIFEST.a2ml │
                        └─────────────────────────────────────────┘
```

Future (not yet implemented, kept out of the diagram deliberately): petgraph
graph engine, tantivy full-text search, Nickel schemas, mobile targets, and the
FL×DT intelligence modules (`index.rs`, `edge.rs`, `reason.rs`, `layout.rs`,
`trigger.rs`) described in [docs/design/](docs/design/).

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
PRODUCT
  Rust Core Engine                  █████████░  90%    Of MVP scope: Note/Notebook, back-
                                                       links, wiki-links, substring
                                                       search, JSON storage, agents,
                                                       Markdown/OPML exchange. 5,344 LOC
                                                       (excl. test modules); 91 tests
                                                       green (82 unit + 3 exchange +
                                                       2 golden + 2 property + 2 doc)
  λδ (LambdaDelta) Substrate        ███████░░░  70%    BUILT, not planned: reader, value
                                                       model, evaluator + Budget sandbox,
                                                       hygienic macros, multimethods,
                                                       prelude, notebook host seam.
                                                       ~3,400 LOC excl. tests (~64% of
                                                       the core) across #35, #36, #43.
                                                       Reachable from JS via
                                                       lambdadeltaEval / evalLambdadelta.
                                                       L1 read-only fx-field UI now ships.
                                                       Outstanding: L2 action hooks,
                                                       L3 REPL, L4 .ld packages
  ReScript UI (TEA-style)           ██████░░░░  60%    Compiles; list/editor work; canvas
                                                       pan/zoom, note drag, dbl-click
                                                       create, keyboard-accessible
                                                       (WCAG 2.2 AA); GraphView renders a
                                                       circular layout; agents panel.
                                                       1,763 LOC. Missing: multi-pane,
                                                       command palette, inspector,
                                                       markdown render, reasoning view
  WASM Bridge (core → browser)      ██████████ 100%    WIRED and exercised: Main.res loads
                                                       the wasm at boot; WasmStore binds
                                                       all 31 exports; ui-ci builds it and
                                                       runs the facade contract test green,
                                                       including all three λδ entry points
  Web / PWA                         ███████░░░  70%    Builds and runs via Bun bundle;
                                                       service worker (offline-first
                                                       precache) + webmanifest + icon
                                                       ship and are registered in
                                                       Main.res. Gap: no automated
                                                       install/offline test
  Desktop Shell (Gossamer)          ░░░░░░░░░░   0%    Blocked-external: needs sibling
                                                       ../gossamer checkout; not buildable
                                                       in this repo

INFRASTRUCTURE
  CI Product Coverage               ███████░░░  70%    Real and load-bearing since #22:
                                                       rust-ci.yml (fmt, clippy
                                                       --all-targets --features wasm,
                                                       tests, wasm32 build) and ui-ci.yml
                                                       (rescript, wasm, Bun tests,
                                                       bundle, lint). SHA-pinned; green
                                                       on main. Gap: no coverage report,
                                                       no browser smoke test, no release
                                                       workflow
  Governance / Meta                 ████████░░  80%    Contractiles, 6a2 STATE, manifest,
                                                       ADRs, design docs, and the
                                                       audience-segmented wiki (#45)

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ███████░░░  ~65%   Core + substrate + bridge are in
                                                       place and green. The critical path
                                                       has MOVED off the WASM bridge and
                                                       onto the UI surface: continuing
                                                       λδ doors (L2→L3)
```

## Key Dependencies

```
Rust Core (incl. λδ) ──► WASM bundle ──► ReScript UI ──► Web bundle (web/dist/)
    │                                              │
    ▼                                              ▼
JSON storage ──► IndexedDB / file download   (optional) Gossamer shell
```

The wasm-bindgen **CLI must match the wasm-bindgen crate version in
`Cargo.lock` exactly** (`just doctor` checks this); a mismatch fails the browser
build with a confusing schema error.

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).

**Percentages are judgement, LOC and test counts are not.** When you revise a
row, re-derive the countable facts rather than copying them forward — this file
understated the core for two weeks because the λδ merges (#35, #36, #43) landed
without a dashboard update, and downstream docs inherited the stale figures.
