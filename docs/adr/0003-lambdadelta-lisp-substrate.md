<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# ADR-0003: LambdaDelta — a Lisp-power substrate for Nexia-List

- **Status:** Proposed
- **Date:** 2026-07-03

> *A note is a letter we send to our future self.*
> — Mark Bernstein, *The Tinderbox Way*

This line is the North Star. Everything below exists to **enlarge** that
correspondence — never to fence it in. Read the framing in the Context section
before the architecture: the power is the servant, the letter is the point.

## Context

Nexia-List is a Tinderbox-like spatial hypertext tool: notes on a canvas,
first-class bidirectional links, attributes, and agents (persistent queries).
Tinderbox's real ceiling is its *expression language* — agent queries, rules,
OnAdd actions, and export templates are written in a small, fixed,
non-extensible mini-language. Users hit its edges and cannot go further.

Nexia-List's notebook is already tree/graph-shaped data (JSON), and the Rust
core is already the single source of truth compiled to WASM (see ADR-0001).
That makes it a natural host for a **homoiconic Lisp** — one where the notebook
*is* data the language can read, and the language's own code is data too. Code
and notebook become the same fabric.

The user's intent, stated plainly:

- Give power users **the full power of Lisp** — not a toy DSL wearing Lisp's
  name. Homoiconicity, closures, recursion, higher-order functions, **macros**,
  a REPL.
- But **not** make Nexia-List "a Lisp app." It remains a Tinderbox-like product,
  usable for a lifetime **without ever seeing a parenthesis**.
- Treat this power **principally as liberating, enabling, and augmenting** —
  the animating purpose (the North Star) is something power *serves and
  enlarges*, not a constraint the product must shrink to fit.

## Decision

Introduce **LambdaDelta (λδ)** — a small, homoiconic Lisp implemented **in the
Rust core and compiled to WASM**, exposing the notebook as first-class data.
λ (functions) + δ (change/transformation): the computational soul that lets a
notebook be programmed *by its owner*, safely, everywhere the tool runs.

Two commitments make it safe and humane:

1. **Progressive power (the UX spine).** Parentheses never appear until a user
   opens a door marked "power." Five optional levels, each layered over the one
   below, none blocking it:
   - **L0 — Tinderbox-basic (no code, ever):** create/drag/link notes; the
     existing simple search + agent DSL. The default, forever.
   - **L1 — Formulas (spreadsheet-gentle):** a computed attribute or agent
     predicate can *optionally* be a λδ expression in a friendly "fx" field with
     autocomplete. The simple DSL still works and quietly compiles to λδ.
   - **L2 — Actions & macros:** note/agent/adornment actions as λδ; user-defined
     functions and `defmacro`, stored *in the notebook*.
   - **L3 — REPL/console:** evaluate against the live graph; define and save
     functions; inspect.
   - **L4 — Beyond:** user-defined commands, views, and exporters;
     **computational notes** (a note whose content evaluates and renders results
     inline); shareable λδ packages.

2. **The letter comes first — as an enabler, not a rule.** Durable,
   human-readable notes are what *make* the augmentation possible: because the
   letter is legible and lasting, computation has something real to amplify.
   Durability is the launchpad, not the leash. The measure of any feature is a
   generative question — *does this help the letter reach a richer future
   self?* — not a veto.

### Architecture (one engine, everywhere)

- **Reader → value model → evaluator** in `core/src/lambdadelta/`. Lexical
  scope, closures, tail calls; special forms `quote`, `if`, `let`, `fn`/`lambda`,
  `define`, `quasiquote`/`unquote`, `defmacro`.
- **Notebook as first-class values.** Builtins expose the whole core surface as
  a Lisp library: `(notes)`, `(note id)`, `(title n)`, `(content n)`,
  `(links n)`, `(backlinks n)`, `(attr n k)`, `(set-attr! n k v)`,
  `(create-note! …)`, `(link! a b)`, `(search q)`, `(run-agent id)`. Pure
  readers vs. `!`-suffixed mutators.
- **Sandbox.** Deterministic; no I/O or network; a step/time/recursion **budget**
  so a user macro can never hang the tab (heavy jobs move to a Web Worker later).
- **Persistence = homoiconicity in action.** Functions, macros, formulas, and
  agent-programs are stored *as data in the notebook*, so they save/load/sync
  with everything else. A notebook carries its own behaviour.
- **Build, don't embed.** A focused interpreter (rather than a general Rust
  Lisp crate) because the whole point is deep notebook-as-data interop and a
  strict browser sandbox — a general embed fights both. No new npm deps
  (Deno-only MUST); implemented in Rust/WASM (no new TS/Python/Go); MPL-2.0.

### Subsumes, does not replace

The merged features become friendly surfaces over the one engine: the current
agents DSL is L0 sugar that compiles to λδ; agent *actions* become arbitrary λδ
programs; computed attributes, prototype inheritance, and (future) adornment
rules are λδ expressions.

## Tinderbox capability coverage

| Tinderbox capability | Under Nexia-List + λδ | Status |
|---|---|---|
| Attributes on notes | Present (`attributes` map); λδ reads/writes | Matched (typing to add) |
| Agents (persistent queries) | Present; queries become λδ predicates | Replaced & exceeded |
| Agent actions | Arbitrary λδ programs (not just Collect/SetAttribute) | Exceeded |
| Rules & Edicts | λδ re-evaluated on change / on schedule | Matched |
| OnAdd / smart containers | λδ action on entry to container/adornment | Matched |
| Adornments (map regions) | Planned; with λδ actions = programmable regions | Exceeded (when landed) |
| Action-code language | λδ *is* this — a real language with macros | Vastly exceeded |
| Stamps | A λδ function applied to the selection | Matched |
| Prototypes / inheritance | `note.prototype` exists; resolution + computed inherited attrs | Exceeded |
| Export templates | λδ functions `notebook → string`; quasiquote templating | Exceeded |
| Links | Bidirectional present; **typed** links | Partial (typing to add) |
| Map / spatial view | Present (drag/pan/zoom) | Matched |
| Outline / Timeline / Attribute-browser / Treemap | Not yet (have list + graph) | Still lacking |
| Typed attributes (number/date/bool/color/set…) | Values are untyped JSON today | Still lacking |
| Attribute-driven visuals (colour/size/badge) | Not yet; λδ can compute style from a formula | Partial → exceeded |
| Rich-text note content | Plain text today | Still lacking |
| Dates / calendar / events | `chrono` in core; no calendar/timeline UI | Partial |
| AppleScript automation (macOS-only) | Replaced by λδ — cross-platform, in-app, sandboxed | Replaced & exceeded |
| Import / Export | OPML + Markdown-vault present; λδ-driven custom formats | Matched → exceeded |

## What will still be lacking (honest gap list)

| Gap | Why it matters | Effort |
|---|---|---|
| Typed attribute system (schemas, per-type editors, validation) | Date math, colours, sets; richer λδ values | M |
| Extra views: outline, timeline, attribute-browser | Parity for non-spatial thinkers | M–L |
| Typed links / link types | Semantic graphs, filtered link queries | S–M |
| Rich-text / markdown rendering in notes | A real writing tool, not plain text | M |
| Attribute/formula-driven visual styling | "See relationships at a glance" payoff | M |
| Computed-attribute dependency tracking + memoization | Efficient recompute at 10k notes | M |
| Off-main-thread (Web Worker) evaluation | Keep the UI fluid under heavy programs | M |
| λδ std-library + package/sharing system | Turns power into an ecosystem | L (ongoing) |
| λδ debugging / tracing / error UX | Power users must see what went wrong | M |
| Desktop shell | Blocked on the external `gossamer` sibling | external |
| Sync / collaboration / mobile | Future / non-goals | L |

## What λδ liberates that Tinderbox cannot match

| Superpower | What it enables | Why Tinderbox can't match it |
|---|---|---|
| Homoiconic notebook-as-data | Metaprogram your knowledge base; generate notes/agents/structure | Its action code can't treat the document as first-class, macro-manipulable data |
| A real language (recursion, HOF, closures, macros) | User-defined DSLs & workflows (GTD, Zettelkasten, research pipelines) as installable packages | Its expression language is bounded and not extensible |
| Computational / literate notes | A note that evaluates and renders results inline | It has export templates, not live in-canvas computation |
| Agents as programs, not just queries | Continuous graph maintenance: indices, invariants, restructuring, generation | Its agents are query + limited action |
| Programs travel with the notebook | Shareable "smart notebooks" that carry their own behaviour; reproducible | Documents carry actions, not a reusable code library |
| One portable, sandboxed automation language | Same automation on web/desktop/mobile — deterministic and safe | AppleScript is macOS-only and out-of-process |
| A moldable tool (Emacs-like) that stays simple by default | Users extend commands, views, exporters — without touching L0 | Not user-extensible at that depth |
| Verifiable substrate | Unit-test notebook programs; property-test invariants | No comparable testable automation model |
| Open · local-first · cross-platform · free | No lock-in; your data and programs are yours | macOS-only, closed, paid |

## Phased rollout (each phase shippable; L0 UX untouched)

- **L0 — Substrate:** reader, value model, evaluator, core special forms,
  notebook builtins, and the safety budget in `core/src/lambdadelta/`;
  wasm-exposed `eval`. Headless and fully tested (reader round-trip, evaluator
  determinism, sandbox limits, property tests). No UX change — the ideal first
  PR because it carries no UI risk.
- **L1 — Formulas:** "fx" fields for computed attributes and agent predicates;
  the simple DSL compiles to λδ.
- **L2 — Actions & macros:** note/agent/adornment actions; `defmacro`; a small
  std-library; user functions stored in the notebook.
- **L3 — REPL/console.**
- **L4 — Computational notes + packages.**

## Consequences

- **Positive:** every existing feature gains a principled, unbounded expression
  layer; the notebook becomes programmable by its owner; automation is
  portable, sandboxed, and testable; the tool becomes moldable while staying
  Tinderbox-simple by default.
- **Costs / risks (with mitigations):**
  - *Browser hang from user code* → step/time/recursion budget; cooperative
    cancellation; Web Worker for heavy jobs later.
  - *Scaring basic users / UX creep* → progressive disclosure; parentheses only
    appear behind an fx/console door; the simple DSL stays the default surface;
    excellent error messages.
  - *Notes↔values interop drift* → one canonical bridge in core, guarded by
    contract tests (as with the existing golden-fixture pattern).
  - *Scope explosion* → L0 is self-contained and verifiable headless; ship
    incrementally.
  - *Stored code longevity* → definitions are data in the notebook; eval-safety
    and versioning on load.

## Relation to other decisions

- Builds on **ADR-0001** (Rust core → WASM, single source of truth).
- Honours **ADR-0002** (Deno-only): the interpreter is homegrown Rust, no new
  JS package-manager surface.
- Respects the `INTENT.contractile` North Star and architectural invariants.
