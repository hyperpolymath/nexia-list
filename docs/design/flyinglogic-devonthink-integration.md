<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Flying Logic × DEVONthink → nexia-list: The Definitive Integration Design

*Status: proposed · Supersedes: three exploratory designs (crown-jewels-first, product-minimalist, substrate-maximalist) and their adversarial review · Audience: nexia-list core contributors · Companion to: ADR-0003 (λδ), the λδ spec v0.1, and the ROADMAP.*

> A note is a letter we send to our future self. This document is about giving that letter two new senses: the ability to **recall its own forgotten neighbours**, and the ability to **reason over its own claims** — both entirely on-device, both invisible until asked for.

---

## 1. TL;DR and the Unifying Thesis

### 1.1 TL;DR

We steal the **intelligence and primitives** of Flying Logic and DEVONthink, not their feature sprawl. Two crown jewels justify genuinely new code in the Rust core; everything else is either already free in nexia's model, or ships later as `.ld` domain packages and Agent-DSL sugar.

| From | Crown jewel | Becomes | New native code |
|---|---|---|---|
| **DEVONthink** | *See Also* — local associative recall | One concordance / inverted index → cosine kNN, BM25 search, Rocchio classify, SimHash dedup, auto-tag | `core/src/index.rs` |
| **Flying Logic** | *Live confidence propagation* — a spreadsheet for reasoning | One pure single-pass DAG sweep over a **separate, additive** typed-edge channel; junctors are ordinary `:type`-tagged notes; operators are a native table a domain pack extends | `core/src/edge.rs`, `core/src/reason.rs`, later `core/src/layout.rs` |

Both are **derived, rebuildable indices that live beside the notes and are never trusted from disk** — the exact discipline nexia already proved with `Notebook::backlinks`. Both surface as **no-parenthesis L0 panels that mount only when non-empty**, and both are reachable as **pure λδ builtins** for power users. Neither is a monolithic subsystem: each is a Rust module + a handful of host builtins + a derived view.

The recommended **first PR** is headless: `core/src/index.rs` alone, property-tested against the existing golden fixtures, with zero UI and zero on-disk format change (§8).

### 1.2 The Unifying Thesis

Both external tools collapse onto **one spine**:

> **A typed, attribute-rich graph + a local intelligence engine + λδ programs — all local-first, all derived-and-rebuildable, all invisible by default.**

The move that unifies them is the move nexia already made for `backlinks`:

1. **A derived index lives beside the notes** and is rebuilt on load, never authoritative on disk (the concordance for DEVONthink; the confidence map and the layered layout for Flying Logic).
2. **Semantics are data, dispatched on `:type`/`:op`** through λδ multimethods (`defmulti … :type`), so operators, entity classes and whole methodologies ship as `.ld` packages without recompiling the core.
3. **The note model is untouched.** `Note.links` stays the untyped associative fabric. A parallel, empty-by-default `edges` channel carries implication. Existing notebooks round-trip byte-for-byte.
4. **We keep the intelligence, not the interpreter tax.** A native operator table is the fast default for the propagation hot path; the λδ `combine` multimethod is the extension path for exotic domain operators only (never called per-node in the inner loop).
5. **Progressive disclosure holds the line.** Typed reasoning is opt-in at L1+, never imposed. Flying Logic's mandatory entity-class discipline is the explicit anti-pattern.

The result: the letter still reaches a richer future self — now able to reason over its own claims and recall its own forgotten neighbours, entirely on-device.

---

## 2. What we are NOT importing (honest non-goals)

These exclusions are load-bearing, not incidental. "Do fewer things well" is a design constraint we enforce, and several of these were flagged by the review as active hazards to avoid.

### 2.1 Categorically out of charter (never build)

| Excluded | Reason |
|---|---|
| **OCR** (image/PDF → text) | Requires Tesseract-class engines and binary pipelines. Content is plain text; Markdown rendering is the only planned rich step. |
| **RSS / feeds, web clipper / Sorter, email import** | Each needs network fetch, OS integration, or proprietary binary parsing — breaks no-network-by-default. Manual Markdown paste is the only sympathetic path. |
| **Multi-device / cloud sync, Server/Sharing edition** | Explicit ROADMAP non-goals (no cloud-only storage, no real-time collaboration). Persistence stays IndexedDB + file import/export; users sync exported JSON/vault themselves. |
| **Rich binary document handling** (PDF annotation, RTFD, web archives) and format-conversion actions | Different product category, heavy WASM burden. The in-scope sliver is Markdown + the existing `exchange.rs` vault. |
| **PDF / PNG / MS-Project diagram export** | Heavyweight rendering plus PM-suite interop, against local-first / no-lock-in. Markdown + OPML already cover interop; at most a client-side SVG snapshot of the layout view. |
| **Mandatory generative AI** (chat, DALL·E, image description), **macOS side-effects** (Speak / Bounce / SendMail), **AppleScript/JXA host** | The λδ sandbox is deliberately no-I/O. λδ (homoiconic, budgeted, in-core) is the scripting escape hatch, not an OS bridge. Local models (Ollama/llama.cpp) are opt-in host capabilities only; no data leaves the device by default. |

### 2.2 In-scope in spirit, but deliberately not built as designed

| Excluded form | What we do instead |
|---|---|
| **Promoting `Note.links: Vec<NoteId>` → `Vec<Link>`** (typed links in the load-bearing field) | Verified to touch ~6 sites (`notebook.rs:75/105/260` backlink loops, wikilink derivation, the `linksto:` agent term, `NoteView.links`, `note_to_value`) and to merge L0 associations with causal edges into one vector. We add a **separate `edges: Vec<Edge>` channel** instead — same capability, near-zero blast radius, byte-identical round-trip. |
| **Per-node λδ multimethod dispatch inside the propagation sweep** | Each `i.apply` decrements the 1M-step `Budget` and readers hold `nb.borrow()`, so a `combine` method touching any mutator panics via `RefCell` double-borrow — and it is materially slower at 10k nodes on every spinner drag. We keep a **native operator table** for the hot path; λδ `combine` is reserved for exotic/domain ops (§4.4). |
| **The six TOC Thinking-Process templates** (CRT, Evaporating Cloud, FRT, PRT, Transition, S&T) as engine features | They are curated domains + starter graphs. Shipping them in core imposes one methodology and violates untyped-by-default. Ship as optional `.ld` domain packs + starter notebooks (data). |
| **Full probability/arithmetic operator sprawl** (Product ×, Sum-Prob ⊕, Proportion ∷, float-flow typing) on day one | Fuzzy min/max/complement + edge weights cover ~80% of reasoning. The rest is trivially added later as `combine` methods; there is no cost to waiting. |
| **A full Dempster–Shafer belief engine** | General DS combination is O(2^\|frame\|) per node with power-set mass functions and conflict renormalisation — non-interactive at 10k. A **DS-lite belief/plausibility interval** is offered as an *optional domain* only (§5.4). |
| **Exotic confidence math as the default** | The default scalar is fuzzy-boolean in [0,1] with 0.5 = Indeterminate. Probability and DS-lite are per-domain opt-ins, never mixed inside one graph (§5.4). |
| **Persisting the concordance or propagation results to disk as authoritative state** | Bloats the human-readable JSON and risks drift. Both are rebuilt on load, exactly like `backlinks`. Derived state is never trusted from disk. |
| **Animated incremental layout, presentation/step-through mode, typed-attribute schemas as a prerequisite** | Pure polish or decoupled workstreams. The static layered view + spinners already deliver the capability; `:confidence`/`:op`/`:weight` ride the existing untyped JSON attrs fine. |

---

## 3. Data-model deltas

The whole delta is: **one new struct + one new field on `Note`**, **three derived indices + a triggers collection on `Notebook`**, and **a few new Agent-DSL predicates**. Everything else rides attributes and `:type`, which the host already surfaces.

### 3.1 The `Edge` struct — a separate, additive channel

`Note.links` (untyped association: wikilinks, backlinks, the L0 graph) **stays exactly as it is** and does **not** participate in confidence flow. Implication lives on a new, empty-by-default field so that a notebook with no reasoning graph serialises identically to today.

```rust
// core/src/edge.rs (new)
use crate::note::NoteId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A typed, weighted, directed implication edge — where Flying Logic's logic lives.
/// `kind` is a bare string so domains add edge classes as DATA, never as a core enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub to: NoteId,

    /// Edge class / dispatch tag: "implies", "supports", "inhibits", "feeds",
    /// "contains", … Default "link" ⇒ a typed edge with no ceremony.
    #[serde(default = "Edge::plain_kind", skip_serializing_if = "Edge::is_plain")]
    pub kind: String,

    /// Influence weight in [-1, 1]. +1 passes through, 0 → Indeterminate (0.5),
    /// -1 negates (Flying Logic's weighting transform). Default +1.
    #[serde(default = "Edge::unit", skip_serializing_if = "Edge::is_unit")]
    pub weight: f64,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, serde_json::Value>,
}

impl Edge {
    fn plain_kind() -> String { "link".into() }
    fn is_plain(k: &str) -> bool { k == "link" }
    fn unit() -> f64 { 1.0 }
    fn is_unit(w: &f64) -> bool { (*w - 1.0).abs() < f64::EPSILON }
}
```

```rust
// core/src/note.rs — ONE new field on `Note`, empty for every existing note
    /// Typed/weighted implication edges (the reasoning graph). Distinct from
    /// `links` (untyped association). Empty ⇒ the note is invisible to the
    /// reasoning engine and serialises to nothing new.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<Edge>,
```

**Why this beats the untagged `Link` enum.** An untagged serde shim on the load-bearing `links` field is the highest-risk round-trip against the golden fixtures (untagged ambiguity, poor error messages). An empty-by-default `Vec<Edge>` guarded by `skip_serializing_if = "Vec::is_empty"` is provably byte-identical for every existing notebook: no new key appears until a note actually has an edge. `Note::new` initialises `edges: Vec::new()`.

**Back-compat proof obligation (Phase gate).** The golden-fixture suite (`golden.rs`) must show that every existing fixture serialises to the same bytes after this field is added. This is a mechanical, checkable gate — not a hope.

### 3.2 Junct / operator / entity / group notes — zero new node primitive

The λδ host already derives `:type` from `attributes["type"]`, and `note_matches` already supports `attr:type=junct`. So junctors, entity classes and groups are **ordinary notes distinguished by attributes** — no schema work.

```jsonc
// An AND-junct as it sits in the notebook JSON — nothing new in the format
{
  "id": "…", "title": "AND",
  "attributes": { "type": "junct", "op": "and" },
  "edges": [ { "to": "<target-uuid>", "kind": "feeds" } ]
}
```

| Concept | Encoding | Surfaced as |
|---|---|---|
| Entity / claim node | `attributes.type = "claim"` (or a domain class) | `:type` |
| Combining operator | `attributes.op = "and" \| "or" \| "not" \| …` | `:op` |
| Asserted driver confidence | `attributes.confidence = 0.8` (f64 ∈ [0,1]) | `:confidence`; editor spinner |
| Entity/edge styling | `attributes.type` / `Edge.kind` | `render` / `edge-style` multimethod |
| Group / collapsible subgraph | `attributes.type = "group"`, members via `:contains` edges | deferred (§8, L4) |

Computed confidences on **driven** nodes are **never persisted** — they would masquerade as asserted drivers and bloat the JSON. Propagation is derived, exactly like `backlinks`.

### 3.3 The concordance / similarity index — the one shared substrate

Rebuildable and incremental, modelled on `backlinks`, but marked `#[serde(skip)]` and **always rebuilt on load** — the strongest possible "don't destabilise L0" guarantee: zero on-disk format change.

```rust
// core/src/index.rs (new) — powers See-Also, Classify, dedup, and BM25 search
use crate::note::NoteId;
use std::collections::HashMap;

type TermId = u32;
type DocId  = u32;   // DENSE note index — the decisive WASM-memory move (§7)

#[derive(Clone, Copy)]
struct Posting { doc: DocId, tf: u32 }

#[derive(Default)]
pub struct SimilarityIndex {
    interner: HashMap<Box<str>, TermId>,
    vocab: Vec<Box<str>>,

    // note ↔ dense id — postings store an 8-byte (DocId, tf), NEVER a 16-byte Uuid
    doc_of: HashMap<NoteId, DocId>,
    note_of: Vec<NoteId>,

    postings: Vec<Vec<Posting>>,      // TermId → postings, sorted by doc
    df: Vec<u32>,                     // document frequency per term

    forward: Vec<Vec<(TermId, u32)>>, // DocId → (term, tf) vector, for reindex + norms
    doc_len: Vec<u32>,                // token count per doc (BM25 |d|)
    doc_norm: Vec<f32>,               // cached L2 norm of the tf-idf vector

    simhash: Vec<u64>,                // near-dup fingerprint, FIXED seed
    content_hash: HashMap<[u8; 32], Vec<DocId>>, // exact-dup buckets (blake3)

    n_docs: u32,
    total_len: u64,                   // for avgdl
}
```

**The dense-`DocId` rule is mandatory, not an optimisation.** Postings must store `(u32, u32)`, never the 16-byte `Uuid`. This is what keeps the index near ~35–50 MB at 10k notes (§7) — comfortable in a browser tab.

Tokenisation is deterministic (Unicode word segmentation → lowercase → stop-list → optional Porter stem), with a **fixed seed** for SimHash and no clock/RNG — honouring the λδ sandbox and IndexedDB determinism. Crates are Rust-only via cargo (`unicode-segmentation`, `rust-stemmers`, `blake3`), so the Deno-only-for-JS MUST is untouched; BM25/cosine/NB are hand-rolled. A hand-rolled ASCII tokenizer + static stop-list is the zero-dep fallback if WASM size ever matters.

### 3.4 `Notebook` deltas

```rust
// core/src/notebook.rs
pub struct Notebook {
    notes: HashMap<NoteId, Note>,

    #[serde(default)]
    backlinks: HashMap<NoteId, HashSet<NoteId>>,   // unchanged (plain links)

    /// Reverse index for TYPED edges, rebuilt exactly like `backlinks`.
    #[serde(default)]
    in_edges: HashMap<NoteId, Vec<EdgeRef>>,       // (source note, edge index)

    /// The concordance. DERIVED — never persisted, rebuilt on load.
    #[serde(skip)]
    index: crate::index::SimilarityIndex,

    #[serde(default)]
    agents: Vec<Agent>,

    /// Smart-Rule triggers (event → predicate → λδ action). Empty by default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    triggers: Vec<crate::trigger::Trigger>,

    pub name: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}
```

`rebuild_backlinks` becomes `rebuild_indices` (keeping the old name as a thin caller): it rebuilds `backlinks`, `in_edges`, **and** `index` from the notes. `WasmNotebook::from_json` already calls the rebuild path, so all three are populated on every load.

### 3.5 Incremental maintenance — hook the write paths, including the title path

`set_content` is the primary write path and already re-derives wikilinks per edit. Reindexing hangs off it:

```rust
pub fn set_content(&mut self, id: &NoteId, content: impl Into<String>) -> Vec<NoteId> {
    let content = content.into();
    // … existing wikilink logic …
    self.index.reindex(*id, &self.title_of(id), &content);   // NEW
    // …
}
```

`reindex` diffs the note's previous forward vector against the new token bag — decrement `df` and remove postings for dropped terms, add postings for new terms, recompute this doc's `doc_len`, `doc_norm`, and `simhash`. Cost is **O(tokens in the edited note)**, not the corpus.

**Resolving the title-path desync (open tension).** Titles dominate short-note similarity, yet `update_title` / `bi_set_title` bypass `set_content`. Both title write paths **must** call `index.reindex(id, new_title, current_content)` too. This is a required part of the first PR's contract, not a follow-up.

**Resolving global-idf / norm drift (open tension).** Every edit changes `df`/`N`, which technically shifts the global idf and thus every *other* note's L2 norm. We do not chase this per-edit. Instead:
- `doc_norm` is cached per note and recomputed only for the edited note on `reindex`.
- See-Also and BM25 compute idf against the **current** `df`/`N` at query time, so ranking is always consistent with the live corpus even though cached norms lag slightly. The lag affects only the *magnitude* of the cosine denominator for unedited notes, never their term content, and `#[serde(skip)]` + rebuild-on-load bounds any accumulated drift to a single session.
- A cheap periodic/opportunistic full `rebuild_indices` (e.g. on notebook open, already free) resets norms exactly. We accept bounded intra-session staleness as the honest, documented trade — it is imperceptible in top-k ordering and it keeps edits O(tokens).

### 3.6 Agent-DSL extensions (DEVONthink Smart Groups already == Agents)

`Agent { query }` **is** a Smart Group, 1:1, already shipping — and replicants are free (§6). We only enrich the parser:

```rust
// core/src/agent.rs — extend `enum Term`
enum Term {
    Text(String), Title(String), Attr(String, String), LinksTo(NoteId), Never,
    Type(String),          // type:junct           — surfaces the FL entity class
    Similar(NoteId),       // similar:<uuid>        — DEVONthink See-Also as a saved query
    Near(NoteId),          // near:<uuid>           — cosine ≥ τ (near-duplicate)
    Conf(Ordering, f64),   // conf:>0.7             — filter by propagated confidence
    Edge(String),          // edge:supports         — has an outgoing typed edge of kind
}
```

`similar:`/`near:`/`conf:` need the index or the propagation result, which the pure `note_matches` (which has no notebook context) cannot supply. **Resolution:** add a notebook-aware sibling `note_matches_in(note, query, &Notebook)` used by `run_query`; the pure `note_matches` stays for index-free terms. The whitespace-ANDed L0 surface is unchanged; these are progressive-disclosure power operators.

### 3.7 Triggers (Smart Rules) — a third `Notebook` collection, like `agents`

```rust
// core/src/trigger.rs (new)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: Uuid,
    pub event: TriggerEvent,
    pub condition: String,   // an Agent-DSL predicate (reuses note_matches)
    pub action: String,      // a λδ Action-context program
    #[serde(default = "yes")] pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerEvent {
    OnCreate, OnSetContent, OnSetAttr, OnLink, OnTag, OnRunAgent,
    Scheduled { every: String },   // "hourly" | "daily" | "workdays" — host-driven tick
}
```

The **local-first subset** of DEVONthink's event taxonomy. Networked events (OnDownload/OnScan/OnOCR/OnSync) are dropped. The host fires a matching trigger's `action` in Action context under a fresh `Budget`. Triggers travel with the notebook, like agents.

---

## 4. The λδ layer

The kernel already implements `defmulti`/`defmethod`, `register_builtin`, a `PRELUDE` eval'd on `Interp::new()`, and `note_to_value` deriving `:type`. We extend the bridge, add builtins, and register a small operator prelude.

### 4.1 Bridge additions (`note_to_value`)

Keep the bridge the single canonical translator; add keys additively (`:links` unchanged):

```rust
// lambdadelta_host.rs :: note_to_value — additive pairs
(Value::kw("edges"),      edges_value(&note.edges)),   // vector of {:to :kind :weight :attrs}
(Value::kw("confidence"), num_or_nil(note.attributes.get("confidence"))),
(Value::kw("op"),         note.attributes.get("op").map(json_to_value).unwrap_or(Value::Nil)),
```

### 4.2 New builtins

**Pure readers** (legal in Formula / Agent-predicate / Agent-query contexts — deterministic, budgeted, no mutation):

| Builtin | Returns | Backed by |
|---|---|---|
| `(see-also n)` / `(see-also n k)` | top-k similar note maps | `index.see_also` |
| `(similar? a b)` | cosine ∈ [0,1] | `index.cosine` |
| `(classify n)` | ranked `[class score]` | Rocchio centroid |
| `(duplicates n)` | exact + near duplicates of `n` | blake3 + SimHash |
| `(search-ranked q)` | notes ranked by BM25 | `index.bm25` |
| `(confidence n)` | asserted-or-computed truth of `n` in [0,1] | `reason::propagate` (memoised per eval) |
| `(propagate)` | map `{#uuid → float}` over driven notes | `reason::propagate` |
| `(drivers)` / `(driven)` | notes with no / some in-edges | reasoning graph |
| `(edges n)` / `(in-edges n)` | typed edges of / into `n` | `Note.edges` / `in_edges` |

**`!`-mutators** (Action context only — budgeted, undoable):

| Builtin | Effect |
|---|---|
| `(set-confidence! id v)` | assert a driver's `:confidence` |
| `(link-typed! from to kind weight)` | push an `Edge` |
| `(junct! op & operand-ids)` → junct id | create `{:type :junct :op op}`, wire operands → junct |
| `(add-tag! id t)` / `(remove-tag! id t)` | maintain `attributes.tags` (a Vec) |
| `(auto-classify! id thresh margin)` | apply top class **only if** `top ≥ thresh ∧ top − runner-up ≥ margin` |

Readers register via `reader(…)`, mutators via `mutator(…)`, sharing the existing `Rc<RefCell<Notebook>>` — line-for-line with the current host bindings.

### 4.3 Evaluation contexts

| Context | Bound | May mutate? | New usage |
|---|---|---|---|
| Formula (pure, `self`) | `self` | no | `(:confidence (propagate))`, `(see-also self 5)`, `(classify self)` |
| Agent-predicate (`self`, truthy) | `self` | no | `similar:`/`conf:`/`type:` compile here |
| Agent-query (→ notes) | — | no | `(filter #(> (first-score (see-also % 1)) 0.6) (notes))` |
| Action (`self`, mutating) | `self` | yes | `(propagate!)`, `(auto-classify! …)`, `(link-typed! …)` |
| **Trigger-action** *(new)* | `self`, **`event`** | yes | Smart-Rule bodies; `self` = triggering note |

The existing L0 agent DSL still compiles to an Agent-predicate λδ expression. `see-also`/`classify`/`propagate` are pure and legal in fx fields (L1); `auto-classify!` and trigger bodies are Action-context, so a probabilistic model can only touch data through the same audited, undoable seam as any mutator.

### 4.4 Multimethod dispatch — pluggable semantics, with a precise native/λδ seam

Three multimethods carry the transferred intelligence and make domains pluggable as `.ld` packages:

```clojure
;; operator algebra — the FL crown jewel's SEMANTICS, extensible without core edits
(defmulti combine (fn [op _inputs] op))
(defmethod combine :and  [_ xs] (reduce min 1.0 xs))    ; weakest link
(defmethod combine :or   [_ xs] (reduce max 0.0 xs))
(defmethod combine :not  [_ xs] (- 1.0 (first xs)))
(defmethod combine :default [_ xs] (reduce min 1.0 xs)) ; entity default = AND-ish

;; rendering — one card per entity/edge class
(defmulti render :type)
(defmethod render :junct [n] {:shape :diamond :label (:op n)})
(defmethod render :claim [n] {:shape :rounded-box :spinner (:confidence n)})

;; edge styling / domain validation
(defmulti edge-style (fn [e] (:kind e)))
(defmulti validate :type)
```

**The seam contract (resolving the open tension explicitly).** The propagation sweep in `reason.rs` runs a **native `Op` table** in Rust for the known operators `{:and :or :not}` (and, once shipped, the probability pack's ops). It calls the λδ `combine` multimethod **only** when a node's `:op` is not in the native table. The contract:

1. **Native is the source of truth for shipped ops.** `combine`'s λδ methods for `:and/:or/:not` exist for REPL inspection and must be *definitionally identical* to the native table (min / max / 1−x). A conformance test asserts native and λδ agree on a sample grid, so crossing the seam never silently changes semantics.
2. **λδ `combine` is never called per-node for a known op.** This is what avoids the verified `Budget` drain and `RefCell` re-borrow panic, and keeps 10k-node spinner drags interactive.
3. **A domain pack adds an op by adding a `combine` defmethod.** Unknown ops resolve through λδ and are accepted as slower; if a domain wants its op on the fast path, it registers a native op id via the host capability (a documented, reviewed extension point), not by patching the evaluator.

This makes the native table a **floor, not a ceiling**: fast by default, infinitely extensible through data.

### 4.5 Domains as `.ld` packages

A *domain* is a `.ld` package that registers a set of `:type`/`:op` tags plus their `combine` / `render` / `validate` / `edge-style` methods and a suggested attribute vocabulary — aligning 1:1 with the kernel/host + plugin-package design (issue #33). Flying Logic's "domain = swappable class+operator pack" and DEVONthink's Classify-by-corpus both become data, not core enums. Shipped examples (§8, L2/L4): `flying-logic.ld` (junct/claim/group types + operator algebra + render cards), `librarian.ld` (confidence-gated auto-classify trigger + kNN auto-tag), and later the TOC methodology packs and a probability operator pack.

---

## 5. Flying Logic mapping

### 5.1 Capability → nexia mechanism

| Flying Logic capability | nexia mechanism | Cost |
|---|---|---|
| Entity = statement node | Existing `Note`; class = `attributes.type` → `:type` | free |
| Directed causal/logical edge | New `Note.edges: Vec<Edge> { kind, weight, attrs }` | S (struct) |
| Junctor (AND/OR bundler) | A note `{:type :junct :op …}`; operands → junct → target via typed edges | free (data) |
| Necessary vs sufficient | `:op :and` = min, `:op :or` = max, selected in `combine` | free |
| Weighted / negated edge | `Edge.weight ∈ [-1,1]` + the weighting transform (w=0→0.5, w<0→negation) | free |
| Confidence scalar | `attributes.confidence: f64`; 0.5 = Indeterminate midpoint | free |
| **Live confidence propagation** | **`reason::propagate` — single-pass DAG sweep; `(confidence n)` / `(propagate)`** | **M (crown jewel)** |
| Advanced operators (XOR/⊕/×/∷) | native `Op` table (hot path) + `defmethod combine :op` for domain ops | S core, ∞ via packs |
| Domains (class + operator packs) | `.ld` packages registering `:type`/`:op` methods + attr vocab | pack (data) |
| Entity classes & styling | `:type` + `defmethod render :type`; prototype supplies class defaults | S |
| TOC thinking-process templates | **out-of-core** domain packs + starter notebooks (§2) | pack |
| Automatic layered DAG layout | new `core/src/layout.rs` (Sugiyama) → `ReasoningView` | M |
| Animated incremental layout | **deferred** — ReScript interpolation only | — |
| Groups / collapse | `{:type :group}` + `:contains` edges; layout/propagate as one node | M (later) |
| Back-edge cycle handling | DFS marks & excludes back edges; drives a "you made a loop" cue | free (in M) |
| Confidence spinners (live) | `SetConfidence` msg → `set-confidence!` → re-`propagate` → re-render | S |
| Fuzzy vs float flow typing | `:flow` tag on computed value; **deferred** | — |
| Operators inspector | settings surface writing `:op` (per-note + notebook default) | S |
| PDF/PNG/MS-Project export | **out** (§2); at most a client-side SVG snapshot | — |

### 5.2 The reasoning-graph view (layered DAG layout)

`core/src/layout.rs` runs a compact Sugiyama pipeline over the typed-edge graph and returns **positions that are computed, never stored** — `Note.position` stays owned by the manual spatial canvas, so auto-layout and free-canvas coexist:

1. Cycle removal — reuse `reason.rs`'s back-edge set (temporarily reverse those edges).
2. Layer assignment — longest-path layering; insert dummy nodes on edges spanning >1 layer.
3. Crossing minimisation — median/barycenter, a few up/down sweeps.
4. x-coordinate assignment — Brandes–Köpf for straight, balanced edges; y from layer index; honour an orientation flag.

Exposed as `reasoning_layout(orient) -> [{id, x, y, layer}]`. It replaces the `GraphView` placeholder and reuses `GraphLayout.res`'s `nodePos` seam — the ReScript view only draws; the layout comes from Rust. Animated transitions are deferred polish.

### 5.3 Confidence propagation — the chosen default semantics

```rust
// core/src/reason.rs
pub type Conf = f32;                              // [0,1]; f32 halves map memory

/// Flying Logic's documented weighting transform, applied per in-edge
/// BEFORE the operator: w=+1 pass-through, w=0 → 0.5, w=-1 → negation.
#[inline]
fn weight_edge(v: Conf, w: f32) -> Conf {
    let s = 2.0 * v - 1.0;      // [0,1] → [-1,1]
    (s * w + 1.0) * 0.5         // → [0,1]
}

#[derive(Clone, Copy)]
pub enum Op { And, Or, Not, /* pack: */ Xor, Product, SumProb, Proportion }

/// Combine already-weighted inputs. Empty input set ⇒ 0.5 (Indeterminate).
fn combine(op: Op, xw: &[(Conf, f32)]) -> Conf {
    if xw.is_empty() { return 0.5; }
    match op {
        Op::And => xw.iter().map(|p| p.0).fold(f32::INFINITY,     f32::min), // weakest link
        Op::Or  => xw.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max),
        Op::Not => 1.0 - xw[0].0,
        Op::Product => xw.iter().map(|p| p.0).product(),
        Op::SumProb => xw.iter().fold(0.0, |a, &(b, _)| a + b - a * b),      // 0.5⊕0.5=0.75
        Op::Proportion => {                                                 // zero-weight abstains
            let (num, den) = xw.iter().fold((0.0, 0.0), |(n, d), &(x, w)| {
                let a = w.abs(); (n + a * x, d + a)
            });
            if den == 0.0 { 0.5 } else { num / den }
        }
        Op::Xor => fuzzy_xor(xw),   // max_i min(x_i, 1 - max_{j≠i} x_j)
    }
}

/// ONE deterministic pass. (1) DFS marks back edges (edges to a node on the
/// stack) and drops them. (2) Kahn topo over the DAG. (3) Sweep: drivers keep
/// their asserted value; driven nodes combine weighted inputs. O(V+E), no fixpoint.
pub fn propagate(nb: &Notebook) -> HashMap<NoteId, Conf> {
    let g = ReasonGraph::build(nb);              // typed edges only, nodes/edges SORTED (§9)
    let back = g.back_edges();                   // DFS over a sorted adjacency
    let order = g.kahn_topo(&back);
    let mut conf = HashMap::with_capacity(order.len());
    for id in order {
        let ins = g.in_edges(id, &back);
        let c = if ins.is_empty() {
            asserted(nb, id).unwrap_or(0.5)       // DRIVER
        } else {
            let xw: Vec<(Conf, f32)> = ins.iter()
                .map(|e| (weight_edge(conf[&e.from], e.weight as f32), e.weight as f32))
                .collect();
            combine(op_of(nb, id), &xw)           // DRIVEN
        };
        conf.insert(id, c);
    }
    conf
}
```

**Why fuzzy min/max weakest-link is the default:**

- **Monotone, order-independent, single-pass.** It evaluates in one topological sweep, O(V+E), with no fixpoint iteration — the only family that stays interactive under spinner-drag at 10k nodes.
- **0.5 = Indeterminate is a better "unknown" than `null`.** A principled, bounded, neutral midpoint that the whole engine runs on.
- **Negation is a number.** The edge-weight transform gives graded support (w<1), abstention (w=0 → 0.5), and logical negation (w=−1) with no extra node type.
- **Back edges are drawable but excluded.** Feedback loops render (dashed, with a "you made a loop" cue) yet never break evaluation, because the evaluated graph is a DAG.

**Made domain-overridable (resolving the semantics tension):** pluggability is layered so two domains can never disagree *inside one graph*:
- The **combine function** is chosen per node by `:op` (native table + λδ extension, §4.4).
- The **scalar semantics** (fuzzy vs probability vs DS-lite interval) is chosen per **domain package**, and a notebook's reasoning graph declares its domain once. A note tagged with a different domain's scalar type is a validation error surfaced by `defmulti validate :type` — the engine refuses to silently mix a fuzzy 0.7 with a DS mass function. Fuzzy is the default when no domain is declared.

### 5.4 Incremental recompute

For spinner drags, cache `order` and each node's downstream-reachable cone; on a driver change, re-sweep only the affected suffix. A full sweep at ~10k nodes / ~30k edges is sub-millisecond in WASM; a single-driver drag touches only its downstream cone.

---

## 6. DEVONthink mapping

### 6.1 Capability → nexia mechanism

| DEVONthink capability | nexia mechanism | Cost |
|---|---|---|
| **See Also** | **`index.see_also` (concordance + TF-IDF + cosine kNN); `(see-also n k)`; L0 "Related notes" panel** | **M (crown jewel)** |
| Classify | `index.classify` (Rocchio centroid; NB as a pack); `(classify n)`; "Suggested tags" chips | M |
| Auto-Classify | `auto-classify!` — apply top class only if `top ≥ τ ∧ top−runner ≥ margin`; Action-context, undoable, opt-in | S (on Classify) |
| **Concordance / inverted index** | **`core/src/index.rs::SimilarityIndex` — the one shared substrate, rebuilt like `backlinks`** | **M (foundational)** |
| Smart Groups (dynamic saved search) | **already shipping** = `Agent { query }`; enrich DSL (`similar:`, `near:`, `conf:`, ranked) | free → S |
| Smart Rules — triggers | `Notebook.triggers`; host fires `TriggerEvent` → predicate → λδ action; local-first subset | M |
| Smart Rules — actions | λδ Action builtins (`set-attr!`, `link-typed!`, `add-tag!`, `auto-classify!`, `create-note!`) | S (mostly exist) |
| Replicants | **free** — one `Uuid`; membership = which agents/tags match; edits shared because one object | free |
| Duplicate detection | `index.duplicates` — blake3 exact + SimHash/LSH near; `(duplicates n)` + panel | S (on index) |
| Tags + hierarchical groups | tags = `attributes.tags` (multi-valued) via `tag:`/`attr:`; hierarchy = agents + prototype | S |
| Full-text search + operators/proximity | BM25 over the index; boolean/phrase/proximity via positional postings; L0 stays whitespace-ANDed | M |
| OCR / rich binary formats / RSS / web-clip / email / sync | **out** (§2) | — |
| Optional generative AI | opt-in λδ host **capability** (Ollama/llama.cpp); never default, no data leaves device | later, opt-in |

### 6.2 The local intelligence engine

One index, five features — the concrete pipeline:

```rust
impl SimilarityIndex {
    /// tf-idf weight = (1 + ln tf) · ln(N / df), idf against CURRENT df/N.
    #[inline] fn w(&self, tf: u32, df: u32) -> f32 {
        (1.0 + (tf as f32).ln()) * ((self.n_docs as f32) / (df as f32)).ln()
    }

    /// See Also: top-k by cosine, self excluded. Walk only the query note's
    /// top ~40 terms; accumulate partial dot-products over their postings.
    pub fn see_also(&self, id: NoteId, k: usize) -> Vec<(NoteId, f32)> {
        let d = self.doc_of[&id];
        let mut score: HashMap<DocId, f32> = HashMap::new();
        for &(t, tf) in top_terms(&self.forward[d as usize], 40) {
            if (self.df[t as usize] as usize) > self.n_docs as usize / 2 { continue; } // freq-stop
            let wq = self.w(tf, self.df[t as usize]);
            for p in &self.postings[t as usize] {
                if p.doc == d { continue; }
                *score.entry(p.doc).or_default() += wq * self.w(p.tf, self.df[t as usize]);
            }
        }
        let mut out: Vec<_> = score.into_iter()
            .map(|(doc, dot)| (self.note_of[doc as usize],
                               dot / (self.doc_norm[d as usize] * self.doc_norm[doc as usize])))
            .collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0))); // deterministic tie-break (§9)
        out.truncate(k);
        out
    }

    /// BM25 (k1≈1.2, b≈0.75) — replaces naive substring search, stands in for tantivy.
    pub fn bm25(&self, query: &str, k1: f32, b: f32) -> Vec<(NoteId, f32)> { /* … */ }

    /// Rocchio nearest-centroid classify: cosine of note to each class centroid.
    /// Cheaper and incrementally updatable vs multinomial NB (offered as a pack).
    pub fn classify(&self, id: NoteId, centroids: &[(ClassId, SparseVec)]) -> Vec<(ClassId, f32)> { /* … */ }

    /// Duplicates: exact via blake3 buckets; near via SimHash Hamming ≤ 3,
    /// candidates gathered by LSH bands (no O(n²) scan).
    pub fn duplicates(&self, id: NoteId) -> Duplicates { /* … */ }
}
```

- **See-Also**: TF-IDF sparse vectors, cosine, top-k walking only the query note's ~40 heaviest terms with a frequency stop-list — single-digit ms at 10k.
- **Classify / Auto-Classify**: each Agent (or tag value) is a class; Rocchio centroid is the default (incrementally updatable), multinomial naive-Bayes ships as a pack. Auto-Classify is the **confidence-gated automation pattern**: apply only when `top ≥ threshold ∧ top − runner-up ≥ margin`, else defer to the human — routed through `auto-classify!` so it is undoable, budgeted, opt-in, never silent.
- **Duplicates**: blake3 exact-hash buckets + 64-bit SimHash (fixed seed) with LSH banding for near-dups; offer merge or "these are the same note".
- **Auto-tag**: kNN label propagation over See-Also neighbours (weighted vote above a threshold) — reuses the similarity engine, no separate model.
- **Ranked search**: BM25 over the same index with positional postings for phrase/proximity. The L0 whitespace-ANDed syntax is the default surface; boolean/phrase/`NEAR/n` are progressive-disclosure power operators.

### 6.3 Smart Rules, Smart Groups, replicants

- **Smart Groups = Agents.** Already shipping, 1:1. The only work is enriching the DSL as the index lands (§3.6).
- **Smart Rules = triggered λδ actions.** `TriggerEvent` → Agent-predicate condition → λδ Action body — DEVONthink's event→condition→action grammar, native to the homoiconic core.
- **Replicants are free.** A `Note` has one `Uuid`; "membership" is which Agents match and which tags it carries. A single note already appears in every matching Agent view and every tag facet, with edits inherently shared because there is one object. Nothing is ever copied.

### 6.4 Positioning note (resolving the tantivy tension)

The ROADMAP commits Phase 3/5 to **tantivy**. We consciously deviate: tantivy gives BM25 but **not** See-Also cosine, Rocchio classify, or SimHash dedup. Running tantivy *and* a separate vector index means two indices and more memory. We therefore **hand-roll one concordance for all five features**, accepting the maintenance burden of our own BM25/phrase/proximity implementation as the price of the shared substrate — a hand-rolled index is comfortably sufficient at 10k notes. This retires the "tantivy planned but not built" seam. We also **reframe ROADMAP Phase 8 "Intelligence"**: See-Also/Classify are the *deterministic local heart* of that phase; any LLM is strictly optional and opt-in, never the default — a positioning this document asks the ROADMAP to adopt.

---

## 7. Algorithms and performance

### 7.1 Concrete choices

| Concern | Choice |
|---|---|
| Tokenisation | Unicode word segmentation → lowercase → stop-list → optional Porter stem; deterministic, fixed data |
| Term weighting | TF-IDF `(1+ln tf)·ln(N/df)`; L2-normalised sparse vectors |
| Similarity | cosine (dot of normalised vectors) |
| Search ranking | BM25, k1≈1.2, b≈0.75, positional postings for phrase/proximity |
| Classify | Rocchio nearest-centroid (default); multinomial NB (pack) |
| Dedup | blake3 exact + 64-bit SimHash (fixed seed) + LSH bands |
| Propagation | edge-weight transform + native operator table; DFS back-edge exclusion + Kahn topo; single pass |
| Layout | Sugiyama (longest-path layering → barycenter → Brandes–Köpf), positions derived |

### 7.2 Complexity and memory at 10k notes (~300 tokens/note, ~200 unique)

| Operation | Cost | Notes |
|---|---|---|
| Full index build | O(Σ tokens) ≈ 3M | tens of ms; on load / import only |
| Incremental reindex (one edit) | O(tokens in note) | in `set_content`/title path; imperceptible |
| See-Also query | O(top-terms × avg df) | 40-term cap + freq-stop → single-digit ms |
| BM25 search | O(Σ df of query terms) | ms |
| Classify (Rocchio) | O(terms(d) × #classes) | ms |
| All-pairs dedup | ~O(N) via LSH bands | not O(N²) |
| Full propagation sweep | O(V+E) ≈ 40k ops | sub-ms in WASM |
| Spinner-drag recompute | O(downstream cone) | interactive |
| **Index memory** | **~35–50 MB** | inverted ≈16 MB + forward ≈16 MB + simhash 80 KB + norms 40 KB. **Requires dense-`DocId` interning** (postings store 8-byte `(u32,u32)`, never 16-byte `Uuid`). Comfortable in a browser tab. |

### 7.3 Incremental update and the off-main-thread escape hatch

All hot operations are incremental: reindex is per-note; propagation re-sweeps only the downstream cone; layout runs on the reasoning subgraph, not the whole notebook. The full build (load, import, or the opportunistic norm-resetting rebuild) is the only O(corpus) operation, and it is tens of ms.

**Web Worker escape hatch.** Everything is pure, deterministic, allocation-only Rust — no threads/network/clock/RNG (SimHash uses a fixed seed) — so it slots into the wasm-bindgen core and honours the λδ budget. If a very large notebook ever makes the initial build or a full rebuild perceptible, the WASM core can be instantiated in a **Web Worker**: the TEA loop posts the notebook JSON in, receives the built indices out, and the main thread never blocks. This is a deployment option, not an architectural change — the core code is identical either way.

---

## 8. Phased roadmap

Aligned to the λδ L0–L4 progressive-disclosure levels and the numbered ROADMAP phases. Each phase is independently shippable; L0 UX (no parentheses, ever) is untouched throughout; new state is either additive-and-skippable or `#[serde(skip)]`-derived.

### The recommended FIRST PR — headless `SimilarityIndex`

**Smallest, headless, highest-value.** Build `core/src/index.rs` + tokenizer + the incremental hook in `set_content` **and both title write paths** + `rebuild_indices`. No WASM entry points, no UI. Unit- and property-tested against the existing `golden.rs` / `invariants.rs` fixtures, including the byte-identical round-trip proof (the index is `#[serde(skip)]`, so this is automatic) and a determinism test (sorted iteration, fixed-seed SimHash). This de-risks the entire DEVONthink side, fills the "tantivy planned but not built" seam, and lands zero user-visible change. It mirrors the L0-substrate discipline ADR-0003 used for λδ.

### Phase map

| Level / ROADMAP | Ships | New state | User-visible |
|---|---|---|---|
| **First PR** (ROADMAP Phase 3) | headless `index.rs`, tokenizer, incremental hooks, `rebuild_indices` | `#[serde(skip)]` index | none |
| **L0** (ROADMAP Phase 3 + differentiator) | See-Also / Suggested-tags / Duplicates panels (mount only when non-empty); BM25 ranked search replaces substring; `Edge` + `reason.rs` + confidence spinner in the editor; `ReasoningView` (`layout.rs`) with driver/driven shading and dashed back-edges | `edges` field (empty by default) | Related panel; ranked search; spinners; reasoning canvas — all no-code |
| **L1** (ROADMAP Phase 4) | pure builtins in fx fields: `(see-also self k)`, `(classify self)`, `(:confidence (propagate))`; Agent DSL gains `similar:`/`near:`/`conf:`/`type:`/`edge:` | — | computed confidence & similarity in formulas |
| **L2** (ROADMAP Phase 4/9) | `(propagate!)`, `(link-typed!)`, `(junct!)`, `(auto-classify!)`, `deftrigger`; `combine`/`render`/`validate`/`edge-style` multimethods; `Notebook.triggers`; ship `flying-logic.ld` + `librarian.ld` | `triggers` collection (empty by default) | Smart Rules; domain packs |
| **L3** | REPL exploration of `(propagate)`, `(see-also …)`, `(duplicates …)`, `(back-edges)` against the live graph under budget | — | power console |
| **L4** (ROADMAP Phase 7 Views + Phase 9 Ecosystem) | computational notes rendering See-Also / propagation inline; layout as a `defview`; TOC / probability / DS-lite domain packs via the plugin system (#33); opt-in local-LLM host capability | — | shareable packages |

**ROADMAP crosswalk.** Phase 3 Search → the concordance + BM25 (retires tantivy). Phase 4 Agents → Smart Groups (exist) + Smart Rules (triggers). Phase 7 Views → layered layout replacing the GraphView placeholder. Phase 8 Intelligence → See-Also/Classify are the deterministic local heart; LLM strictly optional. Phase 9 Ecosystem → both tools as `.ld` domain packages.

**Net new native surface:** one struct + one `Note` field, plus small modules `index.rs`, `edge.rs`, `reason.rs`, `trigger.rs`, and later `layout.rs`. Everything a Flying Logic or DEVONthink user recognises as *intelligence* — operators, domains, methodologies, classification policy, organisation rules, reasoning views — ships as λδ multimethods, `.ld` packages, and Agents on top.

---

## 9. Risks and open questions

**Resolved in this design (documented so the resolution is auditable):**

- **Determinism under the λδ sandbox.** DFS back-edge selection and any HashMap iteration in propagation/index must be order-stable, or golden tests flake and the no-random contract is violated. *Resolution:* the `ReasonGraph` sorts nodes and adjacency lists before DFS/Kahn; See-Also/search sort with an id tie-break (`then(a.0.cmp(&b.0))`); SimHash uses a fixed seed; tokenisation is pure. A determinism property test guards all four.
- **Cycle policy on a graph that had no acyclic invariant.** `links` never required acyclicity and `link_notes` only rejects self-links. *Resolution:* the *new* `edges` channel defines its own policy — DFS-detected back edges are excluded from flow (single pass, no fixpoint), drawn dashed, and surfaced via a "you made a loop" cue. `links` is unaffected.
- **Native/λδ operator seam.** A conformance test asserts the native `Op` table and the λδ `combine` methods agree for shipped ops, so crossing the seam never changes semantics; unknown ops go through λδ and are accepted as slower (§4.4).
- **Back-compat migration.** Empty-`Vec<Edge>` + `skip_serializing_if` gives a provable byte-identical round-trip on every golden fixture — the safest of the three considered approaches, and far safer than an untagged `Link` enum on the load-bearing field.
- **Index memory at 10k.** Mandated dense-`DocId` interning holds it to ~35–50 MB (§7.2).
- **Incremental consistency.** Title paths are hooked; idf/norm drift is bounded to a session by `#[serde(skip)]` rebuild-on-load, with idf computed against current `df`/`N` at query time (§3.5).

**Genuinely open:**

1. **When to trigger the opportunistic full rebuild.** On-open is free; do we also rebuild after N edits, or only when a norm-drift heuristic exceeds a threshold? Needs measurement on real notebooks.
2. **"Promote this association to a typed edge" UX.** The clean split means a wikilink can never also be causal; a user who wants both must create a typed edge. The affordance must avoid silent duplication (the same relation appearing once as a link, once as an edge). Design needed.
3. **Classify class-source ambiguity.** Agents *and* tag values can both be "classes". Which is the default training signal, and how do we present two overlapping class systems without confusing the user?
4. **Stemming/stop-list quality vs determinism across locales.** Porter stemming and English stop-lists markedly improve similarity but are language-specific; the multilingual story (and whether stemming is a per-notebook toggle) is unspecified.
5. **DS-lite domain semantics.** Offering a belief/plausibility interval as an optional domain (§5.4) needs its own operator table and validation rules; the interaction with the native fast path is sketched but not specified.
6. **Web Worker boundary.** If we adopt the off-main-thread build (§7.3), the serialization cost of shuttling a large notebook to the worker must be measured against the main-thread block it avoids.
7. **Trigger execution ordering and loops.** When one trigger's action fires an event another trigger listens for, we need a documented depth/reentrancy limit (the `Budget` bounds a single action, not a cascade).

---

*Steal the intelligence and the primitives, not the feature sprawl. Keep the note model untouched, the semantics as data, the hot path native, the disclosure progressive, and everything on-device. The letter still reaches a richer future self — now able to reason over its own claims and recall its own forgotten neighbours.*