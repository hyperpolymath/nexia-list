<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# TEST-NEEDS.md — nexia-list

## CRG Grade: D (see READINESS.md; last audited 2026-04-15, test matrix updated 2026-08-01)

## Current Test State

| Category | Where | Count | Notes |
|----------|-------|-------|-------|
| Rust unit tests | `core/src/*.rs` (`#[cfg(test)]`) | 85 | domain model, storage, exchange, λδ kernel/host, including generated determinism, macro non-capture, and formula non-mutation properties |
| Rust exchange properties | `core/tests/exchange.rs` | 3 | total import/parser and Markdown topology round-trip |
| Golden contract (Rust) | `core/tests/golden.rs` | 2 | on-disk JSON format, shared fixture `tests/fixtures/notebook.golden.json` |
| λδ conformance (Rust/WASM) | shared JSON fixture + Rust/JS consumers | 1 Rust + 1 JS | same pure-language vectors and printed results on both sides of the WASM boundary |
| Graph/serde properties | `core/tests/invariants.rs` (proptest) | 3 | arbitrary-byte parser totality; invariants after every generated transition, including removed IDs; complete semantic serde round-trip after index rebuild |
| Golden contract (JS/wasm) | `ui/tests/contract.test.js` | 2 | same fixture decoded through the wasm bindings; camelCase view shape; snake_case disk format |
| TEA update tests | `ui/tests/UpdateTests.res` + `update.test.js` | 1 suite (~25 assertions) | CRUD/link/search/zoom/delete-guard against the real wasm core |

## What's Covered

- [x] Core unit tests
- [x] Property-based tests (backlink invariant, serde roundtrip)
- [x] Cross-language contract tests (Rust ⇄ wasm/JS golden fixture)
- [x] UI update-function tests against the wasm core
- [x] CI/CD test automation (`rust-ci.yml`, `ui-ci.yml`)
- [x] Executable proof ledger with explicit claim boundaries
      (`docs/verification/proof-baseline-0.md`)

## Still Missing (for CRG C/B)

- [ ] Browser E2E in CI (Astral smoke flows: create/edit/link/reload-restore,
      keyboard-only session) — verified manually, not yet a CI job
- [ ] Real fuzz target (`fuzz_load_notebook`: arbitrary bytes →
      `serde_json::from_str::<Notebook>` must never panic) replacing
      `tests/fuzz/placeholder.txt`
- [ ] 10k-note performance benchmark (search <100 ms target)
- [ ] Accessibility gate (axe-core in E2E)
- [x] Shared Rust/WASM λδ success-vector corpus
- [ ] Extend the shared λδ corpus to errors, budgets, and formula contexts
- [ ] Canonical serialization, if byte-identical output becomes a requirement

## Run Tests

```bash
bun run test        # cargo test (core) + bun test (UI/contract)
bun run test:rust   # Rust only
bun run test:ui     # UI/contract only (needs build:res + build:wasm first)
```
