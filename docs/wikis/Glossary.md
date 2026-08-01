<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Glossary

Hit an unfamiliar word? This page defines Nexia-List's vocabulary — each term as Nexia-List *actually* uses it. Canonical detail lives in the linked design docs. For orientation, start at [Home](Home) or the [Developer](Developer) page.

---

### Agent
A **saved query** that keeps collecting the notes matching it — Tinderbox's persistent-search idea, and the same thing DEVONthink calls a Smart Group. In Nexia-List an Agent is `Agent { query }`, already shipping; the query DSL grows power operators (`similar:`, `near:`, `conf:`, `type:`, `edge:`) as the [derived index](#derived-index) lands. See [ADR-0003](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0003-lambdadelta-lisp-substrate.md).

### Attribute
A key→value pair on a note (`{:status "todo" :priority 2}`). Attributes are how a note carries structure without a schema — `:type`, `:op`, and `:confidence` all ride the existing untyped attribute map, so junctors, entity classes, and confidence need no new node primitive.

### Backlinks
The reverse index of incoming [links](#link) — "what links to this note?". Nexia already ships it as a derived, rebuilt-on-load map (`Notebook::backlinks`), and it is the **proof-of-concept for the whole architecture spine**: every intelligence feature generalises this one pattern. See [Derived index](#derived-index).

### Classify
Suggesting which class (Agent or tag) a note belongs to, by comparing it to each class's centroid (Rocchio; naive-Bayes as an optional pack). *Auto-Classify* applies the top class only when it clears a confidence-and-margin gate — never silently. DEVONthink's feature, reimplemented in [`index.rs`](#concordance--inverted-index).

### Concordance / inverted index
The one shared substrate behind recall: a term→postings map (plus a forward per-note vector) built in `core/src/index.rs`. From it come [See-Also](#see-also), BM25 search, [Classify](#classify), and duplicate detection. It is a [derived index](#derived-index) — `#[serde(skip)]`, never on disk — and dense `DocId` interning keeps it to ~35–50 MB at 10k notes.

### Confidence propagation
The reasoning engine: a value in `[0,1]` on each node, flowed along typed [edges](#edge) in one deterministic pass over the reasoning graph (`reason.rs::propagate`). Driver nodes keep their asserted value; driven nodes combine their weighted inputs via an operator. Flying Logic's "spreadsheet for reasoning", made native. See [Indeterminate](#indeterminate-05), [Weakest-link](#weakest-link-min).

### Derived index
Any structure computed *beside* the notes, rebuilt on load, and **never trusted from disk** — [backlinks](#backlinks), the [concordance](#concordance--inverted-index), the confidence map, the layered layout. Marked `#[serde(skip)]`. Corrupt it and nothing is lost; it rebuilds. The discipline that makes intelligence a lens rather than a liability.

### DEVONthink
A macOS knowledge manager with the best local associative recall in the business ([See-Also](#see-also), [Classify](#classify)). Nexia-List steals those *algorithms* in portable Rust/WASM — not its document-manager shape — and marries them to a thinking canvas it never had.

### Domain / .ld package
A **`.ld` package** is a bundle of λδ code-as-data that registers `:type`/`:op` tags plus their `combine`/`render`/`validate` methods and an attribute vocabulary — a *domain*. Methodologies (Flying Logic operator packs, DEVONthink classify rules, the six TOC thinking-process trees) ship as data packages, never as core enums. See [LambdaDelta](#lambdadelta-λδ).

### Edge
A **typed, weighted, directed** relationship on a note (`Edge { to, kind, weight, attrs }`) carrying implication — *supports*, *opposes*, *requires*, *causes*… It lives in the additive `edges: Vec<Edge>` channel (empty by default), the place all reasoning happens. Contrast [Link](#link): edges are for argument, links are for association. The two are kept deliberately separate.

### Entity / Claim
An ordinary note that participates in reasoning — a statement whose [confidence](#confidence-propagation) is computed. It is just a note tagged `{:type :claim}` (or a domain class); no new primitive. Its asserted confidence is an [attribute](#attribute).

### Flying Logic
A tool for rigorous fuzzy-logic reasoning over typed graphs — but a standalone diagrammer, disconnected from your notes. Nexia-List brings its *live confidence propagation* into the notebook itself, so junctors and evidence are your actual notes. See [Confidence propagation](#confidence-propagation), [Junct](#junct).

### Gossamer
The **optional** desktop/mobile shell — a linearly-typed webview framework ([hyperpolymath/gossamer](https://github.com/hyperpolymath/gossamer)) that hosts the identical Nexia-List WASM bundle with no port and no type drift. It requires an external sibling checkout and is **not** built in this repo's CI. Desktop and mobile are distribution choices, not separate products.

### Indeterminate (0.5)
The distinguished confidence value meaning *genuinely unknown* — a principled, bounded, neutral midpoint, and a better "unknown" than `null` or a false `0`. A zero-weight edge contributes exactly `0.5`: no information. The whole reasoning engine runs on this scale.

### Junct
A **junctor** — an AND / OR / NOT combiner in the reasoning graph. It is just a note tagged `{:type :junct :op and|or|not}`; its operands feed into it via typed [edges](#edge). `:and` takes the [weakest link](#weakest-link-min), `:or` the strongest, `:not` the complement.

### LambdaDelta (λδ)
The homoiconic Lisp substrate — λ (functions) + δ (change) — implemented in the Rust core and compiled to WASM. It treats the notebook as live data, dispatches behaviour via **multimethods on `:type`/`:op`**, and is **the enabler, never the goal**: a door closed by default, so most people use Nexia-List for life without seeing a parenthesis. The first UI door is a collapsed, read-only note formula panel. See [ADR-0003](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0003-lambdadelta-lisp-substrate.md) and the [spec](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/lambdadelta-spec.md).

### Link
An **untyped association** between notes — a `[[wikilink]]`, a backlink, the L0 graph (`Note.links`). It carries no direction of implication and does **not** participate in [confidence](#confidence-propagation) flow. Kept deliberately distinct from a typed [Edge](#edge); `links` is never promoted to a typed vector (an enforced non-goal).

### Local-first
The core philosophy: your data lives on your device in human-readable JSON, works offline by default, and never requires a server, account, or cloud. Optional export is additive; the file *is* the only sync boundary that will ever exist. Nothing leaves the device.

### Progressive disclosure (L0–L4)
The UX spine: power is arranged as levels you opt into, never imposed. **L0** = plain notes/links/search/canvas, usable for life with zero code; **L1** = data-gated recall/inspector panels; **L2** = command palette, typed edges, reasoning view; **L3** = λδ cells and Smart Rules; **L4** = kernel/host, packages. Two gates only — *data-gated* (a panel mounts iff its set is non-empty) and *door-gated* (`powerLevel >= n`) — never an ad-hoc `if`.

### Proof-obligation ledger
A table mapping each theorem/invariant in the reasoning + index design to a concrete Rust test (`PO-*`). The implementer's job is to turn each row green; a red row names exactly which invariant to look at. The design was adversarially proof-checked before implementation. See [flyinglogic-devonthink-proofs.md](https://github.com/hyperpolymath/nexia-list/blob/main/docs/design/flyinglogic-devonthink-proofs.md) §8.

### Prototype
A note that serves as a template: other notes inherit its [attributes](#attribute) (`Note.prototype`). Tinderbox's inheritance idea; the field exists in the core today, resolution and computed inherited attributes are planned.

### See-Also
Ambient recall: for the note you're looking at, the top related notes by TF-IDF cosine similarity over the [concordance](#concordance--inverted-index) — surfaced as a "Related notes" panel that appears only when non-empty. DEVONthink's crown jewel; the feature that makes the archive write back. Closes the *resurface* stage of the loop.

### TEA
The Elm Architecture — Model / Msg / Update / View with pure update functions and exhaustive pattern matching. Nexia-List's UI is a **hand-rolled** TEA loop in ReScript 11 on `@rescript/react` (the `rescript-tea` library was removed as unused). See [Developer](Developer).

### Tinderbox
The macOS spatial-hypertext tool that inspired Nexia-List — notes on a canvas, [agents](#agent), [prototypes](#prototype), attribute-driven emergence. Its ceiling is a bolted-on, non-extensible formula language and single-vendor, macOS-only mortality. Nexia-List carries its spirit cross-platform, open, and homoiconic to the core.

### Weakest-link (min)
The default semantics for an AND [junct](#junct): a conclusion is only as strong as its weakest support, so `:and` combines inputs with `min`. It is idempotent and single-pass (no fixpoint iteration), which is what keeps [confidence propagation](#confidence-propagation) interactive at scale. `:or` is the dual (`max`).

### WASM
WebAssembly. The Rust core is compiled to a single WASM bundle (via wasm-bindgen) that *is* the engine, running client-side in the browser with no server. See [ADR-0001](https://github.com/hyperpolymath/nexia-list/blob/main/docs/adr/0001-wasm-core-web-first.md).

---

Related: [Home](Home) · [Developer](Developer)
