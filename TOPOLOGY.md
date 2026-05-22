<!-- SPDX-License-Identifier: MPL-2.0 -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-02-19 -->

# Nexia — Project Topology

## System Architecture

```
                        ┌─────────────────────────────────────────┐
                        │              USER INTERFACE             │
                        │        (Spatial Canvas / Notes)         │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           UI LAYER (RESCRIPT)           │
                        │    (TEA Architecture, cadre-tea-router)  │
                        └──────────┬───────────────────┬──────────┘
                                   │                   │
                                   ▼                   ▼
                        ┌───────────────────┐  ┌───────────────────┐
                        │   TAURI SHELL     │  │   BROWSER / PWA   │
                        │ (Desktop/Mobile)  │  │   (WASM/JS)       │
                        └──────────┬────────┘  └──────────┬────────┘
                                   │                      │
                                   └──────────┬───────────┘
                                              │
                                              ▼
                        ┌─────────────────────────────────────────┐
                        │           RUST CORE (CRATES)            │
                        │                                         │
                        │  ┌───────────┐  ┌───────────────────┐  │
                        │  │  Graph    │  │  Search           │  │
                        │  │ (Petgraph)│  │ (Tantivy)         │  │
                        │  └─────┬─────┘  └────────┬──────────┘  │
                        │        │                 │              │
                        │  ┌─────▼─────┐  ┌────────▼──────────┐  │
                        │  │ Storage   │  │  Nickel           │  │
                        │  │ (FS/IDB)  │  │  Schemas          │  │
                        │  └─────┬─────┘  └────────┬──────────┘  │
                        └────────│─────────────────│──────────────┘
                                 │                 │
                                 ▼                 ▼
                        ┌─────────────────────────────────────────┐
                        │             DATA LAYER                  │
                        │  ┌───────────┐  ┌───────────────────┐  │
                        │  │ Local JSON│  │  IndexedDB        │  │
                        │  │ (Files)   │  │  (Web Storage)    │  │
                        │  └───────────┘  └───────────────────┘  │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Justfile Automation  .machine_readable/  │
                        │  Deno Tooling         0-AI-MANIFEST.a2ml  │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
USER INTERFACES
  ReScript UI (TEA)                 ██████░░░░  60%    Spatial canvas prototyping
  Tauri Desktop Shell               ████████░░  80%    Linux/macOS integration verified
  Tauri Mobile Shell                ████░░░░░░  40%    Initial Android stubs
  Web / PWA Deployment              ██████░░░░  60%    Offline service worker verified

CORE ENGINE (RUST)
  Graph Engine (petgraph)           ██████████ 100%    Bidirectional links stable
  Search Indexing (tantivy)         ████████░░  80%    FTS integration refining
  File I/O (JSON/XML)               ██████████ 100%    Local-first storage verified
  Nickel Schemas                    ██████████ 100%    Note validation active

REPO INFRASTRUCTURE
  Justfile Automation               ██████████ 100%    Standard build/setup tasks
  .machine_readable/                ██████████ 100%    STATE tracking active
  Deno Task Runner                  ██████████ 100%    Package management verified

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            ███████░░░  ~70%   Core engine stable, UI refining
```

## Key Dependencies

```
Nickel Schema ───► Rust Core ──────► Graph Engine ──────► Spatial UI
     │               │                 │                   │
     ▼               ▼                 ▼                   ▼
Storage Logic ──► Local Files ──────► Search Index ───► Query Agent
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
