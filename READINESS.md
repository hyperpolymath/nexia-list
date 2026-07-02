<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
<!-- READINESS.md — Component Readiness Grade (CRG) for nexia-list -->
<!-- Last updated: 2026-07-02 -->

# Readiness

**Current Grade:** D

Graded per the hyperpolymath
[Component Readiness Grades](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)
standard. Grade assigned in the
[2026-04-15 post-audit report](docs/reports/audit/audit-2026-04-15-post.md)
(promoted from E/X).

## What D means here

| Aspect | Status |
| --- | --- |
| Builds from source | Yes — `deno task build` (ReScript + esbuild bundle); Rust core builds and its 12 unit tests pass |
| Lockfiles | Yes — `deno.lock`, `Cargo.lock` |
| CI | Estate governance/scanning workflows only; product CI (rust-ci.yml, ui-ci.yml) landing in a parallel workstream |
| Tests | Rust core unit tests only; no UI or integration tests yet |
| Docs | Truth-reset 2026-07-02; roadmap and topology reflect actual state |
| Known debt | unwrap/expect calls in core and desktop; unsafe `get` in View.res (see audit report) |

## Path to C

- Product CI running on every PR (Rust build+test, ReScript build, lint)
- Tests beyond the core crate: UI unit tests exercised in CI
- WASM bridge built and smoke-tested in CI

## Path to B

- Integration tests covering the UI → WASM core → persistence path
- Canvas interaction coverage (pan/zoom, note creation, drag-and-drop once implemented)
- Known-debt items from the audit resolved or explicitly waived
