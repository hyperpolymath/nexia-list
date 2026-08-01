<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Maintainer

You own the roadmap, governance, CI, and the machine-readable state. This page is the *signpost* — the canonical governance lives in [`.machine_readable/`](https://github.com/hyperpolymath/nexia-list/tree/main/.machine_readable) and the repo-root governance files. Edit wiki pages in [`docs/wikis/`](https://github.com/hyperpolymath/nexia-list/tree/main/docs/wikis), never in the forge wiki UI (see [Wiki sync](#wiki-sync)).

## Maintainer's map

| Concern | Canonical source |
|---|---|
| Roadmap (phases) | [ROADMAP.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/ROADMAP.adoc) |
| Unified L0→L4 / P0→P4 sequencing | [docs/design/mind-management-plan.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md) §7 |
| Readiness grading (CRG) | [READINESS.md](https://github.com/hyperpolymath/nexia-list/blob/main/READINESS.md) |
| Architecture map + dashboard | [TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md) |
| Ownership | [MAINTAINERS.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/MAINTAINERS.adoc) |
| Security policy | [SECURITY.md](https://github.com/hyperpolymath/nexia-list/blob/main/SECURITY.md) |
| Changelog | [CHANGELOG.md](https://github.com/hyperpolymath/nexia-list/blob/main/CHANGELOG.md) |
| Contractile gates | [`.machine_readable/`](https://github.com/hyperpolymath/nexia-list/tree/main/.machine_readable) + [`contractiles/`](https://github.com/hyperpolymath/nexia-list/tree/main/contractiles) |
| Test gaps | [TEST-NEEDS.md](https://github.com/hyperpolymath/nexia-list/blob/main/TEST-NEEDS.md) |
| Maintainer quick start | [QUICKSTART-MAINTAINER.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-MAINTAINER.adoc) |

## The phased roadmap

The [ROADMAP.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/ROADMAP.adoc) holds the numbered Phase 0–9 plan. The design synthesis then crosswalks the settled intelligence + UI work into a **unified L0→L4 / P0→P4 sequence** where **every phase is independently shippable, L0 stays no-code throughout, no derived index is ever persisted, and the only new `Note` field (`edges`) serialises to nothing when empty** (full table in [mind-management plan §7](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md)):

| Phase | Ships (summary) | Level |
|---|---|---|
| **P0 — Foundations** | Core-method bindings complete; next: Inspector, Context/Backlinks, theming, PWA manifest, quick-capture/Inbox. **First indexing PR = headless `index.rs`** (zero UI, zero on-disk change) | L0/L1 |
| **P1 — Recall + real editor** | See-Also / Duplicates / tag-suggest panels; markdown render + `[[ ]]` autocomplete | L1 |
| **P2 — Composer** | Command palette; multi-pane; outline/timeline/browser; upgraded canvas (`edge.rs`) | L2 |
| **P3 — Reasoning** | ReasoningView with confidence spinners and dashed back-edges (`reason.rs`, `layout.rs`); Smart Rules (`trigger.rs`) | L2 |
| **P4 — Programmer/Kernel** | Inline λδ cells; `.ld` packages; multimethod authoring; `powerLevel` 3–4 doors | L3/L4 |

## "Do fewer things well" — the do-not-build list

Breadth for its own sake is an explicit non-goal. These are excluded on purpose (carried from [ROADMAP Non-Goals](https://github.com/hyperpolymath/nexia-list/blob/main/ROADMAP.adoc), [mind-management plan §8](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/mind-management-plan.md), and [integration §2](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-integration.md)):

| Excluded | Why |
|---|---|
| OCR, PDF/binary handling, RSS/feeds, web clipper, email import | Need binary pipelines or network fetch; break no-network-by-default |
| Multi-device / cloud sync, real-time collaboration, accounts, subscriptions | The file *is* the sync boundary and it is yours; no lock-in, ever |
| Mandatory generative AI, AppleScript/OS host bridges | The λδ sandbox is deliberately no-I/O; local LLMs are opt-in host capabilities only |
| Promoting `Note.links → Vec<Link>`; persisting derived indices; per-node λδ in the hot loop | Enforced engineering non-goals — see [Developer](Developer) |
| npm/Deno/Yarn/pnpm; new TypeScript/Python/Go source | Bun is the only JS toolchain |

## Governance — contractiles & machine-readable state

Machine-checkable governance is expressed as data, not prose:

| Where | Holds |
|---|---|
| [`.machine_readable/MUST.contractile`](https://github.com/hyperpolymath/nexia-list/blob/main/.machine_readable/MUST.contractile) | Persistent invariants: SPDX on every file, Bun-only, no hardcoded absolute paths, no `unsafe` without a safety comment, tests never weakened, CI SHA-pinned |
| [`.machine_readable/TRUST.contractile`](https://github.com/hyperpolymath/nexia-list/blob/main/.machine_readable/TRUST.contractile) · [`INTENT`](https://github.com/hyperpolymath/nexia-list/blob/main/.machine_readable/INTENT.contractile) · [`ADJUST`](https://github.com/hyperpolymath/nexia-list/blob/main/.machine_readable/ADJUST.contractile) | Provenance/security; the North Star + next-actions; drift tolerances |
| [`.machine_readable/6a2/`](https://github.com/hyperpolymath/nexia-list/tree/main/.machine_readable/6a2) | `STATE`, `META`, `ECOSYSTEM`, `PLAYBOOK` (a2ml) — the honest checkpoint a release reads from; update each session |
| [`contractiles/`](https://github.com/hyperpolymath/nexia-list/tree/main/contractiles) | Root `must` / `trust` / `intend` Verbfiles |
| [`0-AI-MANIFEST.a2ml`](https://github.com/hyperpolymath/nexia-list/blob/main/0-AI-MANIFEST.a2ml) | AI agent entry point (`just llm-context` surfaces role-appropriate context) |

## CI & readiness status

- **Readiness grade: D** ([READINESS.md](https://github.com/hyperpolymath/nexia-list/blob/main/READINESS.md)). Builds from source; lockfiles present; the Rust core's **91 tests** pass. Path to C: a browser smoke test for the WASM bundle, coverage reporting, and a release workflow. Note the grade tracks *release engineering*, not feature completeness — it is not a verdict on how much is built.
- **Overall ~65% MVP** ([TOPOLOGY.md](https://github.com/hyperpolymath/nexia-list/blob/main/TOPOLOGY.md)); the critical path is the UI surface, not the WASM bridge (which is wired and green).
- **CI today** runs *both* estate governance/scanning and product CI: `rust-ci.yml` (fmt, clippy `--all-targets --features wasm`, tests, wasm32 build) and `ui-ci.yml` (ReScript, wasm, Bun tests/bundle, Biome lint), load-bearing since [#22](https://github.com/hyperpolymath/nexia-list/pull/22). Keep every GitHub Action **SHA-pinned**; do not remove a workflow without recorded approval.
- `just check` mirrors `rust-ci.yml` exactly — if it passes locally and CI still fails, that divergence is a bug worth fixing in the Justfile.
- **Local gates**: `just check` (lint + rustfmt + clippy), `just test`, `just doctor`, `just assail` (panic-attacker, if installed).

## Release & packaging

Nexia-List is a **static web bundle** (HTML/JS/CSS + WASM core) with no server-side component — see [QUICKSTART-MAINTAINER.adoc](https://github.com/hyperpolymath/nexia-list/blob/main/QUICKSTART-MAINTAINER.adoc). Build with `just build` + `just build-wasm` (output `web/dist/`); Guix via `guix build -f guix.scm`; container via `stapeln.toml`. The [CHANGELOG.md](https://github.com/hyperpolymath/nexia-list/blob/main/CHANGELOG.md) is generated from conventional commits (Keep a Changelog / SemVer).

## Wiki sync

[`docs/wikis/`](https://github.com/hyperpolymath/nexia-list/tree/main/docs/wikis) is the **single source of truth**; the forge wiki is a published mirror. Publish with the Justfile recipe:

```bash
just wiki-sync dry   # preview what would change, push nothing
just wiki-sync       # commit + push docs/wikis/*.md to the wiki remote
```

It clones the wiki remote (`NEXIA_WIKI_REMOTE` overrides the default), copies the pages over, and pushes a commit stamped with the code repo's short SHA — so every wiki page traces back to the revision it came from. Only the `*.md` pages are mirrored; `README.adoc` stays repo-only. Never hand-edit pages in the wiki UI — the next sync overwrites them.

---

See also: [Home](Home) · [Developer](Developer) · [Glossary](Glossary)
