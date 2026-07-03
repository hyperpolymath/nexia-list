<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-07-02 -->

# Nexia-List — Project Topology

## System Architecture

Nexia-List is **web-first**: the browser is the primary target, running the real
Rust core compiled to WebAssembly. The desktop shell is optional and depends
on an **external** sibling checkout of `hyperpolymath/gossamer`; it is
intentionally not built in this repo's CI.

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
                        │  esbuild bundle → web/dist/             │
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
                        │  Deno 2 tasks   Justfile   scripts/     │
                        │  .machine_readable/  0-AI-MANIFEST.a2ml │
                        └─────────────────────────────────────────┘
```

Future (not yet implemented, kept out of the diagram deliberately): petgraph
graph engine, tantivy full-text search, Nickel schemas, service worker / PWA
install, mobile targets.

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
PRODUCT
  Rust Core Engine                  ████████░░  80%    Of MVP scope: Note/Notebook, back-
                                                       links, substring search, JSON
                                                       storage; 12 passing unit tests
  ReScript UI (TEA-style)           ██████░░░░  60%    Compiles; list/editor work; canvas
                                                       pan/zoom + dbl-click create; no
                                                       drag-and-drop; GraphView placeholder
  WASM Bridge (core → browser)      ████░░░░░░  40%    In progress (wasm-bindgen)
  Web / PWA                         ███░░░░░░░  30%    Builds and runs via esbuild bundle;
                                                       no service worker yet (planned)
  Desktop Shell (Gossamer)          ░░░░░░░░░░   0%    Blocked-external: needs sibling
                                                       ../gossamer checkout; not buildable
                                                       in this repo

INFRASTRUCTURE
  CI Product Coverage               █░░░░░░░░░  10%    In progress: rust-ci.yml + ui-ci.yml
                                                       landing in a parallel workstream;
                                                       existing 11 workflows are estate
                                                       governance/scanning only
  Governance / Meta                 ███████░░░  70%    Contractiles, STATE, manifest, docs

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ████░░░░░░  ~35%   Build resurrected; WASM integration
                                                       is the current critical path
```

## Key Dependencies

```
Rust Core ──► WASM bundle ──► ReScript UI ──► Web bundle (web/dist/)
    │                                              │
    ▼                                              ▼
JSON storage ──► IndexedDB / file download   (optional) Gossamer shell
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
