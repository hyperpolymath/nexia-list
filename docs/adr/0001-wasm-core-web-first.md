<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# ADR-0001: Compile the Rust core to WASM; the browser is the primary target

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

- The desktop shell depends on `hyperpolymath/gossamer` as a path dependency
  (`../../gossamer/bindings/rust`) — an external sibling checkout that is not
  available in this repo or its CI. The desktop layer is therefore not
  buildable here.
- The three layers (Rust core, ReScript UI, shell) were disconnected: the UI
  did not actually call the core, and nothing exercised the seam between them.
- Maintaining separate "desktop core" and "web core" paths invites type drift
  between engine and interface.

## Decision

- Compile `nexia-core` to WebAssembly with wasm-bindgen (`deno task
  build:wasm`).
- The **browser is the primary target**: the bundled app in `web/dist/` runs
  the real Rust core client-side, with persistence via IndexedDB and JSON
  file download/upload.
- The UI delegates all note/notebook semantics to the core through a store
  seam; the ReScript layer holds view state only and never forks the data
  model.
- Desktop becomes an **optional thin shell** added later: Gossamer wraps the
  same web bundle. It stays out of this repo's CI and requires the external
  sibling checkout.

## Consequences

- One engine, one data model — no type drift between platforms; every target
  ships the same tested core.
- The product is buildable and testable entirely within this repo (Deno 2 +
  Rust stable + `wasm32-unknown-unknown` target); CI can cover the real
  product.
- Browser persistence limits apply (IndexedDB quotas; explicit file
  download/upload instead of transparent filesystem access) until a shell
  provides native file I/O.
- WASM boundary costs: data crossing the bridge is serialized, so the API
  surface must stay coarse-grained.
- The desktop experience is deferred; anything desktop-only (native menus,
  file watching) waits for the optional shell.
