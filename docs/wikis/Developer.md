<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Developer Guide

This page is the *signpost* for people hacking **on** Nexia-List — the Rust core, the WASM bridge, the ReScript TEA UI, or the planned λδ substrate and intelligence engine. It tells you where each layer lives and links the canonical design docs; the depth lives in [`docs/design/`](https://github.com/hyperpolymath/nexia-list/tree/main/docs/design) and [`docs/adr/`](https://github.com/hyperpolymath/nexia-list/tree/main/docs/adr), which this page points to rather than repeats.

New here? Read [Home](Home) first, then [QUICKSTART-DEV.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-DEV.adoc) to build and run. This page is for changing Nexia-List *itself*.

## The architecture spine

One idea organises everything intelligent Nexia-List does:

> **A derived, rebuildable index lives beside the notes, is never trusted from disk, and rebuilds on load; behaviour is dispatched on `:type`/`:op` via λδ multimethods.**

This is the discipline Nexia already proved with `Notebook::backlinks` — a reverse index computed from the notes, not authoritative on disk. Every advanced feature generalises that pattern: See-Also, BM25 search, reasoning confidence, layered layout are all `#[serde(skip)]`-derived and rebuilt on load. **The note model stays the primary, human-readable, user-owned data**; the intelligence is a lens you can drop and re-grind. Corrupt an index, lose nothing.

## The stack

| Layer | Technology | Lives in |
|---|---|---|
| Core engine | Rust — Note/Notebook, backlinks, wiki-links, substring search, JSON storage | [`core/`](https://github.com/hyperpolymath/nexia-list/tree/main/core) |
| **λδ substrate** | **Built** — reader, value model, evaluator + Budget, hygienic macros, multimethods, prelude, and the notebook host seam (~3,400 LOC excl. tests, ~64% of the core) | [`core/src/lambdadelta/`](https://github.com/hyperpolymath/nexia-list/tree/main/core/src/lambdadelta), `core/src/lambdadelta_host.rs` |
| Browser bridge | wasm-bindgen (**wired**) — the Rust core compiled to one WASM bundle; all 31 exports bound in ReScript | `core/src/wasm.rs`, [`ui/src/store/WasmStore.res`](https://github.com/hyperpolymath/nexia-list/blob/main/ui/src/store/WasmStore.res) |
| UI | ReScript 11, **hand-rolled** TEA (Model/Msg/Update/View) on `@rescript/react`, Bun bundler | [`ui/`](https://github.com/hyperpolymath/nexia-list/tree/main/ui) |
| Tooling | **Bun only** — packages, tasks, tests, bundler, and dev server | [`scripts/`](https://github.com/hyperpolymath/nexia-list/tree/main/scripts) |
| Persistence | Human-readable JSON via IndexedDB + file download/upload | browser |

Status is tracked live in [TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md) (~65% MVP). The WASM bridge is complete and green locally: all 31 exports, including the three λδ entry points, are bound in ReScript and covered by a facade contract test. The critical path is now giving those capabilities a progressively disclosed UI. The desktop shell ([Gossamer](https://github.com/hyperpolymath/gossamer)) is external and not built in this repo's CI.

## The three tracks

The plan advances on three interlocking tracks that share the spine above (full detail in the [mind-management plan](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) §5):

| Track | What it is | Anchored in |
|---|---|---|
| **A — λδ substrate & moldability** | The homoiconic Lisp base. **Substantially landed**: L0 kernel (reader, value model, evaluator, Budget) in [#35](https://github.com/hyperpolymath/nexia-list/pull/35), the notebook host seam in [#36](https://github.com/hyperpolymath/nexia-list/pull/36), hygienic macros + multimethods + prelude in [#43](https://github.com/hyperpolymath/nexia-list/pull/43). Outstanding: `.ld` packages as data, and surfacing L2+ through the UI's disclosure ladder | [ADR-0003](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0003-lambdadelta-lisp-substrate.md), [λδ spec](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/lambdadelta-spec.md) |
| **B — Local intelligence + typed reasoning** | The settled FL×DT integration: recall (See-Also/BM25/Classify/dedup) + reasoning (typed edges + confidence propagation) | [FL×DT integration](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-integration.md) |
| **C — UI/UX overhaul & cross-platform** | The TEA app grows to multi-pane workspace, palette, inspector, reasoning view, PWA — gated by progressive disclosure, never an ad-hoc `if` | mind-management plan §6 |

## The settled FL×DT integration, in brief

Track B steals the *intelligence and primitives* of DEVONthink and Flying Logic, not their feature sprawl. The decisions are **settled** — the design docs are the spec, not open questions. The whole data-model delta is **one new struct + one new `Note` field**, plus small native modules and a triggers collection:

| Module | Does | Notes |
|---|---|---|
| `index.rs` | One concordance / inverted index → **See-Also** (TF-IDF cosine kNN), **BM25** ranked search, **Rocchio Classify**, **blake3 + SimHash** dedup, kNN auto-tag | `#[serde(skip)]`, rebuilt on load; **dense `DocId` interning** holds it to ~35–50 MB @ 10k notes |
| `edge.rs` | The additive `edges: Vec<Edge{to, kind, weight, attrs}>` channel on `Note` | empty by default, `skip_serializing_if` empty → byte-identical round-trip; `links` stays untyped association |
| `reason.rs` | Pure single-pass DAG `propagate()` — DFS back-edge exclusion + Kahn topo + a **native fuzzy-boolean operator table**; `0.5 = Indeterminate` | junctors/entities are ordinary notes tagged `{:type :junct :op …}` |
| `layout.rs` | Sugiyama layered layout → the **ReasoningView** (replaces the GraphView placeholder) | positions **derived, never written back** to `Note.position` |
| `trigger.rs` | Smart Rules: `event → predicate → λδ action`, the local-first subset | travels with the notebook like agents |

λδ enters only at the seams: the `combine` multimethod for exotic/domain operators (never per-node in the hot loop), Agents as saved queries, and `.ld` packages carrying whole methodologies as data. Native is the **floor**, λδ the **no-ceiling** extension.

### The recommended FIRST PR

Headless **`index.rs`** alone: tokenizer + inverted index + incremental reindex hooks (on `set_content` **and both title write paths**) + `rebuild_indices`, property-tested against the golden fixtures. **Zero UI, zero on-disk change.** It de-risks the entire recall side before a pixel ships, and mirrors the L0-substrate discipline ADR-0003 used for λδ. The subsequent PR order is in the [integration doc](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-integration.md) §8 and the [mind-management plan](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) §7.

### Correctness & proofs

The reasoning and index designs were **adversarially proof-checked** against the real `core/` (a v2 pass corrected four subtle bugs: float folds must be pinned in key-sorted order for determinism; byte-identity needs canonical map serialization; the posting `remove` must be two-sided; the load path must call `rebuild_indices`). Each theorem and invariant maps to a concrete Rust test in the **proof-obligation ledger** — implement against the invariant, then turn the `PO-*` row green. See [flyinglogic-devonthink-proofs.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-proofs.md) (§8 is the ledger; §9 is the handoff order).

## Enforced non-goals (do not "improve" these)

These are load-bearing constraints, not oversights:

- **Do not promote `Note.links` → `Vec<Link>`.** Typing lives in the additive `edges` channel; `links` stays untyped association. Promoting it touches ~6 sites and merges L0 association with causal edges.
- **No per-node λδ dispatch inside the propagation sweep.** It drains the budget and double-borrows the `RefCell`; the native operator table is the hot path, `combine` is for exotic ops outside the loop.
- **Never persist derived indices to disk.** backlinks, See-Also, BM25, reasoning layout are all `#[serde(skip)]`, rebuilt on load.
- **Bun only**; markdown is Rust `pulldown-cmark` → typed AST → ReScript. No npm/Deno/Yarn/pnpm, no new TypeScript/Python/Go files.
- **No parenthesis before a door**; no L2+ surface at `powerLevel == 0`; no L1 panel that renders an empty header.

The full list is [mind-management plan §8](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) and [integration §2](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-integration.md).

## Key design docs

| Doc | Why read it |
|---|---|
| [docs/design/mind-management-plan.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) | Canonical product + engineering synthesis: thesis, the loop, tracks A/B/C, disclosure ladder, unified roadmap, non-goals |
| [docs/design/flyinglogic-devonthink-integration.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-integration.md) | The settled FL×DT architecture — data-model deltas, algorithms, phased PRs |
| [docs/design/flyinglogic-devonthink-proofs.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-proofs.md) | The proofs + the proof-obligation ledger the implementer discharges |
| [docs/design/lambdadelta-spec.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/lambdadelta-spec.md) | λδ language spec v0.1 — syntax, note-as-value, builtins, kernel/host seam |
| [docs/adr/0001-wasm-core-web-first.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0001-wasm-core-web-first.md) | Why Rust→WASM, web-first |
| [docs/adr/0004-bun-only-toolchain.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0004-bun-only-toolchain.md) | The Bun-only toolchain rule |
| [docs/adr/0003-lambdadelta-lisp-substrate.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0003-lambdadelta-lisp-substrate.md) | Why a homoiconic Lisp substrate, progressive-power L0–L4 |

## Building & contributing

Three commands from [QUICKSTART-DEV.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-DEV.adoc):

```bash
just setup          # bun install + rustup wasm32 target
just build          # ReScript compile + Bun bundle   (build-wasm for the core)
just test           # cargo test (core) + bun test (UI)
```

Before a PR: `just check` (Biome + rustfmt --check + clippy), `just test`, `bun run fmt`. Read [CONTRIBUTING.md](https://github.com/hyperpolymath/nexia-list/blob/main/CONTRIBUTING.md) and the invariants in [`.machine_readable/MUST.contractile`](https://github.com/hyperpolymath/nexia-list/blob/main/.machine_readable/MUST.contractile) first — every source file needs an SPDX header (MPL-2.0 code, CC-BY-SA-4.0 docs), tests must not be weakened, and CI workflows stay SHA-pinned.

---

See also: [Home](Home) · [Maintainer](Maintainer) · [Glossary](Glossary)
