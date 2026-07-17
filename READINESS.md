<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- READINESS.md — Component Readiness Grade (CRG) for nexia-list -->
<!-- Last updated: 2026-07-17 -->

# Readiness

**Current Grade:** D

> **Re-audit due — this grade is probably stale.** Every criterion listed under
> *Path to C* below now appears to be met (see the checklist). The grade is
> assigned by audit, not by this file, so it is left at D pending a maintainer
> re-audit rather than self-promoted. `just crg-badge` publishes this letter, so
> the badge is understating the project until that happens.

Graded per the hyperpolymath
[Component Readiness Grades](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)
standard. Grade assigned in the
[2026-04-15 post-audit report](docs/reports/audit/audit-2026-04-15-post.md)
(promoted from E/X).

## What D means here

| Aspect | Status |
| --- | --- |
| Builds from source | Yes — `deno task build` (ReScript + esbuild bundle); Rust core builds and its **90 tests** pass (81 unit + 3 exchange + 2 golden + 2 property + 2 doc) |
| Lockfiles | Yes — `deno.lock`, `Cargo.lock` |
| CI | Estate governance/scanning **plus product CI**: `rust-ci.yml` (fmt, clippy `--all-targets --features wasm`, tests, wasm32 build) and `ui-ci.yml` (ReScript, wasm, Deno tests, bundle, lint) — both on every PR, SHA-pinned, green on main |
| Tests | Rust core (90) **and** UI (10, via `deno task test:ui`), including a TEA↔WASM contract test. No browser-level integration test yet |
| Docs | Truth-reset 2026-07-02; TOPOLOGY re-derived 2026-07-17 after the λδ merges (#35, #36, #43) had gone unrecorded |
| Known debt | unwrap/expect calls in core and desktop; unsafe `get` in View.res (see audit report) |

## Path to C

All three appear **met** as of 2026-07-17 — a re-audit should confirm and regrade:

- [x] Product CI running on every PR (Rust build+test, ReScript build, lint) — `rust-ci.yml` + `ui-ci.yml`, both `on: pull_request`
- [x] Tests beyond the core crate: UI unit tests exercised in CI — 10 tests via `ui-ci.yml`
- [x] WASM bridge built and smoke-tested in CI — `rust-ci.yml` builds wasm32 + generates bindings; `ui-ci.yml` runs the `TEA update delegates to the wasm core` contract test against the real bundle

## Path to B

- Integration tests covering the UI → WASM core → persistence path (the UI → WASM half is covered; the IndexedDB persistence half needs a browser-level test)
- Canvas interaction coverage (pan/zoom, note creation, drag — all now implemented; geometry is unit-tested, direct interaction is not)
- Known-debt items from the audit resolved or explicitly waived
