<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# TEST-NEEDS.md — nexia-list

## CRG Grade: D (see READINESS.md; last audited 2026-04-15, test matrix updated 2026-07-02)

## Current Test State

| Category | Where | Count | Notes |
|----------|-------|-------|-------|
| Rust unit tests | `core/src/*.rs` (`#[cfg(test)]`) | 14 | note, notebook (incl. self-link rejection, backlink rebuild), storage |
| Golden contract (Rust) | `core/tests/golden.rs` | 2 | on-disk JSON format, shared fixture `tests/fixtures/notebook.golden.json` |
| Property tests | `core/tests/invariants.rs` (proptest) | 2 | backlinks == exact inverse of links after any op sequence; serde roundtrip. Found a real self-link index-corruption bug on first run. |
| Golden contract (JS/wasm) | `ui/tests/contract.test.js` | 2 | same fixture decoded through the wasm bindings; camelCase view shape; snake_case disk format |
| TEA update tests | `ui/tests/UpdateTests.res` + `update.test.js` | 1 suite (~25 assertions) | CRUD/link/search/zoom/delete-guard against the real wasm core |

## What's Covered

- [x] Core unit tests
- [x] Property-based tests (backlink invariant, serde roundtrip)
- [x] Cross-language contract tests (Rust ⇄ wasm/JS golden fixture)
- [x] UI update-function tests against the wasm core
- [x] CI/CD test automation (`rust-ci.yml`, `ui-ci.yml`)

## Still Missing (for CRG C/B)

- [ ] Browser E2E in CI (Astral smoke flows: create/edit/link/reload-restore,
      keyboard-only session) — verified manually, not yet a CI job
- [ ] Real fuzz target (`fuzz_load_notebook`: arbitrary bytes →
      `serde_json::from_str::<Notebook>` must never panic) replacing
      `tests/fuzz/placeholder.txt`
- [ ] 10k-note performance benchmark (search <100 ms target)
- [ ] Accessibility gate (axe-core in E2E)

## Run Tests

```bash
deno task test        # cargo test (core) + deno test (UI/contract)
deno task test:rust   # Rust only
deno task test:ui     # UI/contract only (needs build:res + build:wasm first)
```
