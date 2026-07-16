<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Nexia-List

**A local-first tool for the real management of mind — Tinderbox-inspired spatial notes with a Rust→WASM core, a ReScript TEA UI, and a λδ (LambdaDelta) programmable substrate.**

Welcome to the Nexia-List wiki. This wiki is the *signpost* — it orients each reader and points to the canonical docs; it does not duplicate them. The source of truth is [`docs/wikis/`](https://github.com/hyperpolymath/nexia-list/tree/main/docs/wikis) in the code repo. **Edit the Markdown there, never in the forge wiki UI** — the UI copy is a published mirror and gets overwritten.

---

## Start here

| If you want to… | Go to |
|---|---|
| Read the full pitch | [README.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/README.adoc) |
| See the phased plan | [ROADMAP.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/ROADMAP.adoc) |
| See the architecture map + completion dashboard | [TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md) |
| Understand the product vision (canonical) | [docs/design/mind-management-plan.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) |

---

## Pick your track

| You are a… | Read |
|---|---|
| **User** managing your own mind with Nexia-List | [User](User) |
| **Developer** hacking on the Rust core, WASM bridge, or ReScript UI | [Developer](Developer) |
| **Maintainer** owning the roadmap, governance, and CI | [Maintainer](Maintainer) |
| **Curious / non-technical** reader (press, funders) | [Lay-Public](Lay-Public) |
| Anyone hitting an unfamiliar term | [Glossary](Glossary) |

---

## What Nexia-List is

Nexia-List is an instrument for the **real management of mind** — capturing, structuring, linking, **reasoning over**, recalling, and **resurfacing** decades of thought — that runs entirely on your device, shows nothing you didn't ask for, and works the same everywhere. It is not a place to *store* notes; it is a place where your notes keep working on your behalf. The category is crowded with capture-and-store tools that abandon the reader exactly where a mind is actually managed — **reason** and **resurface**. That neglected second half of the loop is the product.

The mind-management loop it serves:

```
CAPTURE → STRUCTURE → LINK → REASON → RECALL → RESURFACE → PUBLISH
   ▲                                                          │
   └────────────────────  the future self  ──────────────────┘
```

Today's tools are strong at capture and link and effectively broken at **reason** and **resurface** — the two stages Nexia-List treats as first-class. The thesis, the loop, and the positioning are set out in [docs/design/mind-management-plan.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md).

## Component map

| Layer | Technology | Role |
|---|---|---|
| Core engine | Rust (serde, uuid, chrono) | Note/Notebook model, backlinks reverse index, substring search, JSON storage |
| Browser bridge | wasm-bindgen (in progress) | Compiles the Rust core to a single WASM bundle that *is* the engine, client-side |
| UI | ReScript 11 + hand-rolled TEA on `@rescript/react`, esbuild | Model / Msg / Update / View; type-safe functional UI |
| Substrate (planned) | λδ (LambdaDelta) — homoiconic Lisp in the Rust core | Notebook-as-data; multimethods on `:type`/`:op`; opt-in, invisible by default |
| Desktop/mobile shell (optional) | [Gossamer](https://github.com/hyperpolymath/gossamer) — external sibling | Thin webview wrapping the identical web bundle; not built in this repo's CI |

Tooling is **Deno 2 only** (no npm/bun/yarn/pnpm); persistence is human-readable JSON via IndexedDB + file download/upload.

## Project status

Nexia-List is **~35% of the way to its MVP** (build resurrected; the WASM bridge is the current critical path). Per [TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md): Rust core ~80% of MVP scope, ReScript UI ~60%, WASM bridge ~40%, Web/PWA ~30%, desktop shell blocked-external. Readiness grade **D**; see [READINESS.md](https://github.com/hyperpolymath/nexia-list/blob/main/READINESS.md). Treat TOPOLOGY as the live dashboard.

## Relationship to other projects

- **[Gossamer](https://github.com/hyperpolymath/gossamer)** — the optional desktop/mobile webview shell; a sibling checkout hosts the identical WASM bundle with no port. Not required, not in this repo's CI.
- **LambdaDelta (λδ)** — the homoiconic Lisp substrate planned in-core; see [ADR-0003](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0003-lambdadelta-lisp-substrate.md) and the [spec](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/lambdadelta-spec.md).
- **[Tinderbox](https://www.eastgate.com/Tinderbox/)** — the inspiration for spatial notes, agents, and prototypes (macOS-only, closed); Nexia-List carries the spirit cross-platform, open, and homoiconic.

## Governance

- **Licence**: MPL-2.0 for code, CC-BY-SA-4.0 for documentation ([LICENSE](https://github.com/hyperpolymath/nexia-list/blob/main/LICENSE)).
- **Machine-readable state & contractiles**: [`.machine_readable/`](https://github.com/hyperpolymath/nexia-list/tree/main/.machine_readable) (`MUST`/`TRUST`/`INTENT`/`ADJUST` contractiles, `6a2/` STATE/META/ECOSYSTEM) and the root [`contractiles/`](https://github.com/hyperpolymath/nexia-list/tree/main/contractiles) verb files; AI entry point [`0-AI-MANIFEST.a2ml`](https://github.com/hyperpolymath/nexia-list/blob/main/0-AI-MANIFEST.a2ml).
- **Security**: [SECURITY.md](https://github.com/hyperpolymath/nexia-list/blob/main/SECURITY.md). **Issues**: [github.com/hyperpolymath/nexia-list/issues](https://github.com/hyperpolymath/nexia-list/issues).
