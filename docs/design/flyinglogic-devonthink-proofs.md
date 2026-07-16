<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Nexia-List — Hard Problems, Solved, with Proof Designs

*The fable-tier work. Everything below is the reasoning that is expensive to get right and cheap to get subtly wrong. It is written so that a lower-tier implementer (Opus) can translate each module into Rust/ReScript against a fixed spec and discharge a checkable **proof-obligation ledger** (§8) — without re-deriving any mathematics.*

Companion to: the FL×DT integration design and the mind-management plan. Assumes their decisions (additive `edges` channel; derived-index-beside-the-notes discipline; native operator table + λδ `combine` for exotic ops; `.ld` domains).

> **v2 — corrections after an adversarial proof-check (Opus) against the real `core/`.** Four items were materially wrong in v1 and are fixed inline (each tagged ⚠v2): **(1)** float folds must be pinned in key-sorted order — IEEE non-associativity means "sort the outputs" is *not* enough for determinism (§1.4, §2.4, §6); **(2)** byte-identical serde is false until the pre-existing `notes`/`backlinks`/`attributes` `HashMap`s serialize in canonical key order (§3); **(3)** the `O(1)` posting `remove` must be *two-sided* or reindex corrupts Invariant L (§2.1); **(4)** the driver/driven boundary is strictly by `G'` in-degree — `combine(∅)` is dead code (§1.4/§1.6). The mathematical core (§1.2–1.7, §2.2–2.3 ideas, the `in_edges`/`:not` catches) survived. The ledger (§8) gained the discriminating tests.

---

## 0. The delegation contract

| Tier | Owns | Must NOT need to do |
|---|---|---|
| **fable (this document)** | the data-structure *shapes* that make the complexity claims true; the invariants; the theorems and their proofs; the caught bugs; the proof-obligation ledger | — |
| **Opus (implementation)** | translate each `§ spec` block into Rust/ReScript honouring the stated invariants; write the test named in each `PO-*` ledger row until green; wire WASM entry points and TEA views | re-derive the math, invent the data structure, or decide the semantics |

**Reading order for Opus:** for each module, read the *Invariant*, then the *Spec*, then implement, then make the *Proof Obligations* pass. The proofs here exist so that when a test fails, the implementer knows which invariant was violated rather than guessing.

Three bugs in the prior design were found while proving it; they are called out inline as **⚠ CAUGHT**.

---

## 1. Confidence propagation — the reasoning engine

The claim to be justified: *one deterministic pass computes a well-defined confidence for every node; feedback loops are drawn but never break evaluation; a spinner drag recomputes only what can change.*

### 1.1 Setup and notation

- The reasoning graph is `G = (V, E)` where `V` = notes and `E` = typed edges (`Note.edges`), each edge `(u→v)` carrying weight `w ∈ [-1,1]`. `links` do **not** appear here.
- A node is a **driver** if it has no in-edges in the (reduced) graph; otherwise **driven**.
- `conf : V → [0,1]`. `0.5` is the distinguished value *Indeterminate*.
- `asserted(v) ∈ [0,1]` is a driver's stored `:confidence` (default `0.5`).
- `op(v)` is the node's combining operator (default `:and`; a junct carries `:op`).

### 1.2 Theorem (Reduced-DAG). *Removing the canonical DFS back-edge set from `G` yields a DAG.*

**Proof.** Run DFS over `G`. Classify each edge as tree / forward / cross / back, where `(u→v)` is **back** iff `v` is on the recursion stack (grey) when the edge is explored — i.e. `v` is a DFS-ancestor of `u`. Assign finish times `f`. For every non-back edge `(u→v)`, standard DFS parenthesis theory gives `f(u) > f(v)`. Let `E' = E \ Back`. Order `V` by *descending* `f`. Every edge of `E'` then goes from an earlier to a later vertex, so this is a topological order of `(V, E')`; a graph admitting a topological order is acyclic. ∎

### 1.3 Theorem (Determinism of the cut). *With a canonical DFS the back-edge set — hence `E'`, the topo order, and every `conf(v)` — is a pure function of the notebook.*

**Why it is not free:** which edge of a cycle is the back edge depends on DFS visitation order, and Rust `HashMap` iteration order is unspecified (and, in `wasm32`, must never be relied on). 

**Proof / construction.** Build `ReasonGraph` as **sorted vectors**, never a `HashMap` walked for order:
- nodes in ascending `NoteId` order (`Uuid` has a total order);
- each adjacency list sorted by `(target NoteId, source edge index)`.
DFS pushes roots and neighbours in that fixed order. Finish times, and therefore `Back`, are now a deterministic function of the sorted structure, which is itself a deterministic function of the notebook value. Every downstream quantity is a pure function of `Back`. ∎

> **§ spec.** `ReasonGraph::build(nb)` returns `{ ids: Vec<NoteId> (sorted), idx: HashMap<NoteId,u32>, adj: Vec<Vec<EdgeRef>> (each sorted) }`. `back_edges()` is an iterative DFS over `adj` using an explicit `Vec` stack and a `Vec<Color>` — no recursion (WASM stack safety), no `HashMap` iteration.

### 1.4 Theorem (Well-defined, order-independent values). *On `G' = (V, E')` the assignment*
```
conf(v) = asserted(v)                                          if in-deg_{G'}(v) = 0
conf(v) = combine(op(v), { (weight_edge(conf(u), w), w) : (u→v) ∈ E' })   otherwise
```
*has a unique solution, and one Kahn sweep computes it regardless of which valid topological order Kahn happens to pick.*

**Proof.** `G'` is a DAG (§1.2), so the definition is a well-founded recursion: `conf(v)` refers only to `conf(u)` for strict predecessors `u`, and the predecessor relation is a strict partial order with no infinite descending chains. Well-founded recursion has a unique total solution. Kahn processes each `v` only after all predecessors are finalized, so it evaluates exactly the recursion. Uniqueness of the *solution* (not merely of one run) gives order-independence: any linear extension yields the same `conf`, because each `conf(v)` is fixed by the values of its predecessors, which are fixed inductively from the drivers up. ∎

**Determinism caveat — `combine` must be a *canonical* function of the inputs.** Two distinct hazards:

- *Multiset symmetry.* min, max, `1−x`, fuzzy-xor are order-exact.
- **⚠v2 CAUGHT (float non-associativity).** IEEE `+`/`×` are commutative but **not associative**, so `prob-sum` (`1−∏(1−xᵢ)`), `product` (`∏xᵢ`) and `proportion` (`Σ|wᵢ|xᵢ / Σ|wᵢ|`) are *not* functions of the input multiset in `f32`: a different fold order yields a ULP-different result, and a single ULP flips a `total_cmp` tie in a ranked list (§2.4) or changes a serialized `conf` byte (§6). **Mandatory spec line:** `combine` folds its inputs in **sorted `(source NoteId, source edge-index)` order** — a pure function of the notebook. (`min/max/not/fuzzy-xor` are order-exact and need no pinning, but pinning all uniformly is simplest.) This is what makes `PO-1.4a`/`PO-6` achievable; without it they fail intermittently.

`:not` is unary — the remaining symmetry exception:

> **⚠ CAUGHT (arity).** `:not` is unary. A `:not` node with in-degree ≠ 1 is undefined. Resolution: `validate` (§4.3) rejects it at edit time; `combine(:not, xs)` asserts `xs.len()==1` and is only ever reached for validated graphs. Document `:not` as strictly unary in `flying-logic.ld`.

### 1.5 Lemma (Edge-weight transform). *`weight_edge(v, w) = ((2v−1)·w + 1)/2` maps `[0,1]×[-1,1] → [0,1]` and realizes pass-through / abstain / negation.*

**Proof.** `2v−1 ∈ [-1,1]`; with `|w|≤1`, `(2v−1)w ∈ [-1,1]`; `+1 → [0,2]`; `/2 → [0,1]` (closure). Substituting: `w=1 ⇒ v` (identity); `w=0 ⇒ 0.5` (abstain — a zero-weight edge contributes Indeterminate, i.e. *no information*); `w=−1 ⇒ 1−v` (negation). `∂/∂v = w`, so monotone increasing for `w>0`, decreasing for `w<0`, flat at `w=0`. ∎

### 1.6 Lemma (Operator closure). *Every shipped operator maps `[0,1]^k → [0,1]`.* 

**Proof (the only non-obvious cases).**
- **prob-sum** `⊕`: `a⊕b = a + b − ab = 1 − (1−a)(1−b)`. For `a,b∈[0,1]`, `(1−a)(1−b)∈[0,1]`, so `a⊕b∈[0,1]`; associativity gives the `k`-ary form `1 − ∏(1−xᵢ) ∈ [0,1]`. ∎
- **proportion** `∷`: `Σ|wᵢ|xᵢ / Σ|wᵢ|` is a convex combination of values in `[0,1]` ⇒ in `[0,1]`; `Σ|wᵢ|=0 ⇒ 0.5` by definition. ∎
- **fuzzy-xor**: `maxᵢ min(xᵢ, 1 − max_{j≠i} xⱼ)` — a max of mins of values in `[0,1]` ⇒ in `[0,1]`. ∎
- min / max / `1−x` / product: closure immediate.

**⚠v2 — driver/driven boundary (resolving a v1 inconsistency).** Classification is **strictly by `G'` (post-cut) in-degree**: a node with `in-deg_{G'}(v) = 0` is a **driver** and takes its `asserted` value (default `0.5`) — *even if it had in-edges in `G` that were all back-edges*. Consequently `combine` is only ever invoked with ≥1 surviving input; `combine(∅)` is **unreachable dead code** (if defensively kept, return `0.5`). Practical upshot: a note carrying `:confidence = 0.8` that is targeted only by cyclic edges reports **0.8**, not `0.5` — its assertion is honoured, not silently discarded (`PO-1.6b`).

### 1.7 Theorem (Incremental recompute — the spinner-drag correctness). *If only driver `d`'s `asserted` value changes, the set of nodes whose `conf` can change is exactly the descendants of `d` in `G'`. Re-sweeping `descendants(d)` in topo order recomputes them correctly and leaves all other nodes untouched.*

**Proof.** `conf(v)` is determined (§1.4) by the asserted values of the drivers that are ancestors of `v`. If `d ∉ ancestors(v)` then none of `v`'s determining inputs changed, so by induction over the topo order restricted to non-descendants, `conf(v)` is unchanged. Conversely every `v ∈ descendants(d)` has `d` as an ancestor and may change. Re-sweeping precisely `descendants(d)` (a subset closed under "successor", processed in the global topo order) evaluates the same recursion for exactly the affected nodes. ∎

> **§ spec.** Cache `order: Vec<NoteId>` and, per driver, `cone: Vec<NoteId>` (its forward-reachable set, computed lazily by BFS over `adj` and memoized until the edge set changes). `set_confidence!(d,x)` → re-sweep `cone(d) ∩ order` in order. A full sweep is `O(V+E)` (~40k ops at 10k notes/30k edges) — sub-millisecond in WASM — so the cone optimization is a UX nicety for very large graphs, not a correctness dependency.

### 1.8 Semantics choice (why fuzzy min/max weakest-link is the default), stated as properties

The default algebra is the one family that is simultaneously: **(P1)** closed on `[0,1]`, **(P2)** symmetric (so §1.4 determinism holds), **(P3)** evaluable in a single pass with no fixpoint (min/max are idempotent and need no iteration to converge), and **(P4)** has a neutral `0.5` that is a genuine "unknown" rather than a false `0`. Probability (`×`, `⊕`) and DS-lite intervals are opt-in per-domain (§4.3) precisely because they trade one of these away (e.g. `×` is not idempotent, so repeated evidence double-counts — desirable sometimes, wrong as a default).

---

## 2. The similarity index — data-structure invariants and incremental correctness

The claim to be justified: *one inverted index supports See-Also / BM25 / Classify / dedup; a single-note edit costs `O(tokens in that note)`, not `O(corpus)`; results are exact and deterministic; deletion is handled.*

### 2.1 The `O(tokens)` reindex — the doubly-linked posting structure (the real trick)

> **⚠ CAUGHT (complexity).** The prior design stores postings "sorted by doc" and claims `O(tokens)` incremental reindex. Removing one doc's posting from a sorted `Vec` is `O(df)` (shift), and there is no way to *find* it without an `O(df)` scan — so the honest cost of the naive structure is `O(Σ_t df(t))`, which for common terms is `O(N)`, not `O(tokens)`. The `O(tokens)` claim is only recoverable with a different structure.

**The fix — unsorted postings with two-way back-links.** See-Also and BM25 both accumulate scores into a `HashMap<DocId, f32>` by *iterating* a term's postings; neither needs them sorted (only positional phrase/proximity needs per-doc position lists, which live elsewhere). So postings can be unsorted, which unlocks `O(1)` add/remove:

```rust
struct Posting { doc: DocId, tf: u32, fwd: u32 }   // fwd = index into forward[doc]
struct FwdEntry { term: TermId, tf: u32, slot: u32 } // slot = index into postings[term]

// INVARIANT L (link): for every term t and every slot s,
//   let p = postings[t][s];   forward[p.doc][p.fwd] == FwdEntry{ term: t, .., slot: s }
// and symmetrically for every forward entry. The two arrays are mutual inverses.
```

- **add(d,t,tf):** `s = postings[t].len(); j = forward[d].len();` push `Posting{d,tf,fwd:j}`; push `FwdEntry{t,tf,slot:s}`; `df[t]+=1`. `O(1)`.
- **remove(d, term at forward index `j`):** **⚠v2 — this is a *two-sided* swap-remove.** The arrays are mutual inverses, so *both* must be fixed:
  1. *postings side:* `s = forward[d][j].slot`; `swap_remove(postings[term], s)`; if a posting `P` moved into slot `s`, set `forward[P.doc][P.fwd].slot = s`.
  2. *forward side:* `swap_remove(forward[d], j)`; if a `FwdEntry Q` moved into index `j`, set `postings[Q.term][Q.slot].fwd = j`.
  `df[term] -= 1`. Both fixups are `O(1)` (each guarded by a "was anything actually moved?" check for the last-element case). **Omitting step 2 corrupts Invariant L on the *reindex* path** (a surviving doc dropping a term); §2.2 deletion hides the omission only because it `clear()`s `forward[d]` wholesale (`PO-2.1`).

Therefore `reindex` = (diff old vs new token bag) then `O(1)` per changed `(doc,term)` = **`O(|B_old| + |B_new|)`**, genuinely `O(tokens in the edited note)`. Invariant L is the checkable contract (PO-2.1).

### 2.2 DocId lifecycle under deletion (the gap the design left open)

**Decision: tombstone during a session, compact on load.** DocIds are allocated append-only (`0,1,2,…`) as notes are indexed. Deleting note `d`:
1. for each `FwdEntry` in `forward[d]`: `remove(d, term)` (§2.1), which maintains `df`;
2. `forward[d].clear(); note_of[d] = None; doc_of.remove(&note_id); simhash[d] = DEAD; n_docs -= 1;`
3. DocId `d` is **never reused** within the session.

**Theorem (deletion preserves INV-IDX).** After deletion, (a) no posting references `d`; (b) `df[t]` equals the number of live docs containing `t`; (c) `n_docs` equals the live count; (d) every other doc's postings are unchanged *in content* (swap-remove only relabels slots, preserving Invariant L). **Proof.** (a) every term of `d` had its posting removed in step 1; (b) each removal decremented exactly the terms `d` contained; (c) step 2; (d) `swap_remove` moves a posting but §2.1's fixup restores Invariant L, and its `(doc,tf)` payload is untouched. ∎

**Memory bound.** Tombstones (holes in `note_of`) number at most the session's deletions. Because the index is `#[serde(skip)]` and **rebuilt on load** from the sorted live notes, DocIds are re-densified every session ⇒ holes never accumulate across sessions. This is exactly the `backlinks` discipline: *derived, never trusted from disk, rebuilt on load.*

### 2.3 Retiring the idf/norm-drift tension — exact cosine at query time

> **⚠ CAUGHT (the drift was avoidable).** The prior design caches `doc_norm` under stale idf and then argues the resulting ranking error is "imperceptible." We can do better: compute the exact cosine and delete the tension.

`cos(q,d) = ⟨w_q, w_d⟩ / (‖w_q‖·‖w_d‖)` with tf-idf weight `w(t) = (1+ln tf)·ln(N/df(t))` under the **current** `N, df`. See-Also already forms the numerator `⟨w_q,w_d⟩` by walking `q`'s top terms' postings and accumulating into `score[d]`, all with current idf. The only question is the denominator norms:

- `‖w_q‖` — computed once from `q`'s forward vector under current idf: `O(|terms(q)|)`.
- `‖w_d‖` for each candidate `d` — computed from `d`'s forward vector under current idf: `O(|terms(d)|)`.

**Theorem (exactness).** Computing numerator and both norms all under the current `(N, df)` yields the exact current cosine; ranking candidates by it yields the exact top-k. **Proof.** Immediate — every term of the definition is evaluated under one consistent idf. ∎

**Cost — the honest bound (⚠v2).** Let `C` = candidate set (docs sharing one of `q`'s top-40 terms after dropping terms with `df > N/2`). Extra work is `Σ_{d∈C}|terms(d)|`. **Worst case is `O(N)` / `O(corpus)`**: `|C| ≤ 40·(N/2) = O(N)`, so exact-at-query-time is an *average-case* win (real top terms are discriminative ⇒ `C ≪ N`), **not** a worst-case guarantee. **Guaranteed path:** when `Σ_{t∈top40} df(t)` exceeds a fixed budget `B`, fall back to cached `doc_norm` so the query is always `O(B)`; also tighten the stop rule (absolute posting cap, or `df > N/10`). The fallback error is a per-doc scaling: `score_cached(d) = score_exact(d)·ρ_d`, `ρ_d = ‖w_d‖_cache/‖w_d‖_exact → 1` as term `df` grows, `|ρ_d−1| = O(E/(df_min·ln(N/df)))`, `E` = edits since that doc's last reindex (reset to 0 on load). So: **exact under budget `B`, cached-norm above it** — the fallback is a guaranteed path, not merely a profiling flag. Norm and numerator folds are `TermId`-ordered (§2.4).

### 2.4 Determinism of the intelligence pipeline

Every order-sensitive step is pinned:
- **tokenization** is pure (Unicode segmentation → lowercase → static stop-list → optional Porter stem; all fixed data);
- **SimHash** uses a **fixed seed** (no RNG);
- **See-Also / BM25** sort by `score` then `NoteId` tie-break: `b.score.total_cmp(&a.score).then(a.id.cmp(&b.id))`;
- **no `HashMap` is iterated** where output order matters (accumulate into a map, then sort the entries);
- **⚠v2 — float reductions fold in key-sorted order** (the subtle one): the cosine numerator `Σ_t w_q(t)·w_d(t)` and both norms `Σ_t w(t)²` iterate terms in **`TermId` order** (and the top-40 cut breaks ties by `TermId`); `combine`'s `proportion`/`prob-sum`/`product` fold in-edges in `(source NoteId, edge-index)` order (§1.4). Because IEEE `+`/`×` are non-associative, **sorting the *outputs* is not sufficient — the *accumulation* order must itself be a key-sorted, notebook-pure function**, or a 1-ULP difference flips a `total_cmp` tie between two near-equal docs and breaks `PO-2.4`/`PO-6`. Classify's centroid dot-products fold the same way (`PO-2.6`).

Consequence (with §1.3): the entire index is a pure function of the live note set, independent of insertion order — golden tests are stable and the λδ no-RNG/no-clock contract holds.

### 2.5 Near-duplicate recall (SimHash + LSH), as a bound not a hope

64-bit SimHash; near-dup iff Hamming distance `≤ h₀` (e.g. 3). Split the 64 bits into `b` bands of `r = 64/b` bits; two docs are LSH-candidates iff some band is identical. For a pair at Hamming distance `h`, the probability a given band matches is `((64−h)/64)^r`… (standard bit-sampling bound); with `b=8, r=8`, pairs at `h≤3` collide in ≥1 band with probability `≥ 1 − (1 − (61/64)^8)^8 ≈ 0.999`. Choose `(b,r)` from the target `h₀` and acceptable false-negative rate; exact duplicates (Hamming 0) are caught additionally by blake3 content-hash buckets (zero false negatives). Determinism: fixed seed + `NoteId`-sorted candidate output.

### 2.6 Master index invariant + per-op maintenance

```
INV-IDX ≜  Invariant L (§2.1)                                     (postings ↔ forward are inverses)
        ∧  ∀t. df[t] = #{ live d : t ∈ bag(d) }
        ∧  n_docs   = #{ live d }
        ∧  ∀ live d. forward[d] = tfidf-bag(title(d) ++ content(d))  (under tokenization)
        ∧  ∀ live d. simhash[d] = SimHash(bag(d))    (fixed seed)
```
| Op | Maintenance |
|---|---|
| `create` | allocate DocId, `add` each term, set `simhash`, `n_docs+=1` |
| `set_content` | `reindex(id, title(id), new_content)` (diff) |
| `set_title` (**incl. `update_title`/`bi_set_title`**) | `reindex(id, new_title, content(id))` — **⚠ CAUGHT earlier:** titles dominate short-note similarity; both title write paths must call reindex, or See-Also silently desyncs |
| `delete` | §2.2 |
| `set_attr` / link ops | no index change (attrs/links are not indexed text) |

---

## 3. Serde back-compat — round-trip fidelity, and the two things that break byte-identity

> **⚠v2 — revised after verification.** The v1 claim ("any current notebook round-trips byte-identically once new fields carry the right attributes") is **false as stated**, for a reason *independent of the new fields*. Corrected below.

**What is actually required (and provable).** Existing notebooks must (a) **load without loss** under the extended schema, and (b) gain **no new JSON keys** while their new fields are empty/derived. This holds iff every new field is `#[serde(skip)]` (derived) or `#[serde(default, skip_serializing_if="…is_empty")]`, declared after the existing fields.

**Proof (of the required property).** A missing key deserializes via `default`/`skip` (no effect on siblings); an empty/derived value is omitted on serialize (no new key). ∎ This is sufficient for correctness and for forward/backward loading — it is the property Opus must guarantee.

**Why *byte*-identity needs one more fix (⚠v2 — the bigger bug).** The *existing* struct already serializes three `HashMap`s in **iteration order**:
- `notebook.rs:26` `notes: HashMap<NoteId, Note>`
- `notebook.rs:29` `backlinks: HashMap<NoteId, HashSet<NoteId>>`
- `note.rs:70` `attributes: HashMap<String, Value>`

`serde_json` emits maps in iteration order, and `std::HashMap`/`HashSet` iterate in a per-instance seed-dependent order, so `serialize → deserialize → serialize` **permutes keys** for any map with ≥2 entries (deserialize builds a fresh map with a fresh seed). Byte-identity was therefore **never actually held** — the current round-trip test (`notebook.rs:394`) checks only semantic fields, never bytes. Adding correctly-attributed new fields is *necessary but not sufficient*.

**Fix (only if byte-level golden gates are wanted).** Serialize the three maps in **canonical key order** — least-invasive is `#[serde(serialize_with = "sorted_map")]` (keeps `HashMap` in memory, sorts on the way out); alternatively switch to `BTreeMap`/`BTreeSet` (larger blast radius — touches `note.rs`, `agent.rs::attribute_equals`, `exchange.rs`) — and **regenerate the golden fixtures** with the canonical serializer.

**Theorem (corrected).** *Under canonical map ordering, any current notebook round-trips byte-identically once new fields carry the attributes below.* Proof exactly as above, now that key order is a pure function of the key set. ∎

If byte-level gates are *not* wanted, relax `PO-3` to **semantic** round-trip equality (the property that truly matters) and skip the canonical-serialization change. **Recommendation:** adopt `serialize_with` canonical ordering — cheap, makes golden diffs meaningful, and removes a latent cross-platform flake.

**The `in_edges` sub-bug (real, independent of the above).** The prior delta declared `in_edges` with `#[serde(default)]` and **no** `skip_serializing_if`; an empty map still serializes as `"in_edges":{}`, injecting a key. `backlinks` (`notebook.rs:29`, `#[serde(default)]`, **persisted**) is living proof of exactly this behaviour. Fix — `in_edges` is derived, so `#[serde(skip)]` it and rebuild. **Wording correction:** this *deviates* from the `backlinks` discipline (which *is* persisted), rather than "following it" — we are choosing not to persist any derived reverse index. `#[serde(skip)]` also requires `SimilarityIndex: Default`.

```rust
#[serde(skip)]                                          index: SimilarityIndex,  // derived; needs Default
#[serde(skip)]                                          in_edges: HashMap<NoteId, Vec<EdgeRef>>, // ← was the bug
#[serde(default, skip_serializing_if="Vec::is_empty")]  triggers: Vec<Trigger>,
// on Note:
#[serde(default, skip_serializing_if="Vec::is_empty")]  edges: Vec<Edge>,
```

**⚠v2 — load-path rebuild (caught by verification).** `#[serde(skip)]` fields deserialize to `Default` (empty). `rebuild_backlinks` (`notebook.rs:257`) is **not** called by serde on deserialize — today only `from_markdown_vault` (`exchange.rs:159`) calls it. So the documented load entry point (`WasmNotebook::from_json`) **must** call `rebuild_indices` after `serde_json::from_str`, or `index`/`in_edges`/`backlinks` are silently empty after a plain load (`PO-3b`).

---

## 4. The native / λδ seam — memory safety and semantic conformance

### 4.1 Lemma (No double-borrow during propagation). *Evaluating an exotic-op `combine` method inside the sweep cannot panic on `RefCell`.*

**The hazard, precisely.** Host readers do `nb.borrow()` (shared); host mutators do `nb.borrow_mut()`. A `borrow_mut()` while any shared borrow is live is a `BorrowMutError` **panic**. The sweep holds a shared borrow (or an owned snapshot); if a λδ `combine` method could call a `!` mutator, it would `borrow_mut()` and panic.

**Proof it cannot.** Evaluation contexts form a capability lattice: `Reader = {Read}`, `Action = {Read, Mutate}`. Every builtin is registered with a required capability; all `!` builtins require `Mutate`. The dispatcher evaluates `combine` in **Reader** context, whose capability set lacks `Mutate`, so any `!` builtin call resolves to a *capability-denied error value* (not a panic, not a borrow) **before** any `borrow_mut()`. Therefore no `borrow_mut()` occurs during the sweep; only nested shared borrows occur, which `RefCell` permits. ∎

**Belt-and-suspenders (recommended).** `propagate` operates on the `ReasonGraph` **snapshot** (plain `Vec`s), not on `nb` directly. Then exotic `combine` methods that merely *read* the snapshot don't even nest a `RefCell` borrow. Both mechanisms are cheap; ship both.

**⚠v2 — three interpreter invariants the proof depends on (made explicit after verification).** The capability argument is airtight *iff*: **(I1) capability monotonicity** — a nested `eval` (via higher-order builtins `map`/`reduce`/`do`/`let`) inherits the caller's capability set and can never *escalate* to Action; **(I2) totality** — every `!` builtin is registered requiring `Mutate` (no unlabelled mutator); **(I3) no native bypass** — no native fn reachable from `combine` calls `nb.borrow_mut()` directly, outside the capability gate. Without I1, a `combine` calling `(map f xs)` re-enters eval, and a `!` inside `f` under an escalated context would `borrow_mut` while the sweep holds `borrow()` → panic. These are interpreter invariants, discharged by `PO-4.1b`. The snapshot removes the hazard only for methods holding **no** ambient `Rc<RefCell<Notebook>>` handle.

### 4.2 Conformance (native table ≡ λδ methods for shipped ops)

Native `Op::{And,Or,Not,…}` and the λδ `(defmethod combine :and …)` etc. both implement one written spec `S(op)`. This is discharged as a **proof obligation**, not a closed proof: a property test over a deterministic grid `Gᵏ = ({0, 1/16, …, 1}‥ arity≤4)` asserts `|native(op,x) − eval_λδ(op,x)| ≤ 1e-6` for every shipped `op`. Passing certifies the seam is invisible: a node routed native vs λδ yields the same number. Unknown ops route to λδ and are accepted as slower (§4 of the design). **The dispatch is a total partition** `op ↦ Native(f) | Lambda`, native consulted first, so the two never both fire for one node (determinism of routing).

### 4.3 Domain scalar-isolation invariant (two domains can't disagree inside one graph)

```
INV-DOM ≜  notebook declares ≤1 reasoning :domain D (absent ⇒ fuzzy default)
        ∧  ∀ reasoning node n. op(n) ∈ ops(D) ∧ type(n) ∈ types(D)
```
Maintained by `(defmulti validate :type)` on every `set_attr`/`link_typed`/junct creation: an `:op` or `:type` outside `ops(D)/types(D)` is rejected with a diagnostic. **Consequence:** `propagate` only ever sees operators from a single algebra with a single scalar semantics (fuzzy `[0,1]` vs DS-lite interval), so a fuzzy `0.7` can never be combined with a DS mass function. Mixed graphs are unrepresentable, not merely discouraged.

---

## 5. Trigger cascade termination

**Default: firing-suppression.** A thread-local guard `IN_TRIGGER` is set while a trigger action runs; host mutators check it and **do not enqueue** new trigger events while set. **Theorem:** each host-originated event fires each matching trigger at most once, and with a finite trigger set the reaction terminates in one pass. **Proof.** No event is generated during action execution ⇒ the event queue for a tick is fixed at the tick's start ⇒ bounded by `|events| × |triggers|`. ∎

**Optional bounded cascade** (for users who *want* reactions to react): a worklist with a `visited: HashSet<(TriggerId, NoteId)>` dedup and a hard depth ceiling `D`. Terminates because the state space `(TriggerId × NoteId)` is finite and never revisited, and the `Budget` still bounds each individual action.

---

## 6. Master determinism theorem (the keystone)

**Theorem.** *For a fixed notebook value `NB` (fixed ids, text, edges, attrs), each of `build_index(NB)`, `propagate(NB)`, `reasoning_layout(NB)`, `see_also/bm25/classify/duplicates` is a pure, deterministic function of `NB`, independent of `HashMap` iteration order and of any prior in-memory state.*

**Proof.** By §1.3 (sorted `ReasonGraph`), §2.4 (sorted tie-breaks, fixed SimHash seed, pure tokenization, no order-bearing `HashMap` iteration), and the observation that `layout` consumes the same sorted `ReasonGraph` and the same `Back` set as `propagate` (so the dashed-drawn edges are exactly the flow-excluded edges — a shared-source consistency property). Each stage is a composition of sorted iteration and pure functions, hence invariant under insertion order; composition of deterministic functions is deterministic. ∎

**Corollaries.** (i) Golden/property tests are stable across runs and platforms. (ii) The λδ sandbox's no-RNG/no-clock contract is honored *by construction* for all reasoning/intelligence builtins (note *creation* uses `Uuid::new_v4`/`Utc::now`, but that is host-side note authoring, outside evaluation). (iii) A Web-Worker build produces bytewise-identical indices to a main-thread build — so §7's off-main-thread option in the design is safe to adopt without behavioral change.

---

## 7. Complexity ledger (the claims, now earned)

| Operation | Cost | Earned by |
|---|---|---|
| full index build / rebuild-on-load | `O(Σ tokens)` | one pass |
| **incremental reindex (one edit)** | **`O(tokens in note)`** | §2.1 doubly-linked postings — *not* the sorted-Vec structure |
| See-Also query (exact) | `O(Σ_{t∈top40} df(t) + Σ_{d∈C}|terms(d)|)` | §2.3 |
| BM25 search | `O(Σ df of query terms)` | inverted index |
| Classify (Rocchio) | `O(|terms(d)|·#classes)` | centroid dot products |
| dedup | `~O(N)` via LSH bands | §2.5 |
| full propagation sweep | `O(V+E)` | §1.4 single pass |
| spinner-drag recompute | `O(|cone(d)|)` | §1.7 |
| index memory @10k | ~35–50 MB | dense `DocId` (8-byte postings), never `Uuid` |

---

## 8. Proof-obligation ledger — the checkable gates Opus makes green

Each row is a theorem/invariant above and the concrete Rust test that discharges it. When a row is red, the named invariant is the place to look.

| PO | Discharges | Test (property unless noted) |
|---|---|---|
| **PO-1.2** | Reduced-DAG (§1.2) | after `back_edges` removal, `kahn_topo` consumes all nodes on random digraphs |
| **PO-1.3** | Determinism of cut (§1.3) | shuffle adjacency *insertion* order ⇒ identical `back_edges` and `order` |
| **PO-1.4** | Order-independence (§1.4) | many random valid topo linearizations ⇒ identical `conf` map |
| **PO-1.4a** | **float-fold determinism (§1.4)** | permute **in-edge insertion order** for a `proportion`/`prob-sum`/`product` node of in-degree ≥3 ⇒ **bit-identical** `conf` (fails unless the fold is `(source NoteId, edge-idx)`-sorted) |
| **PO-1.4b** | `:not` arity (§1.4 ⚠) | a `:not` node with in-deg≠1 fails `validate`; never reaches `combine` |
| **PO-1.5** | weight transform (§1.5) | unit `w∈{-1,0,1}`; monotonicity sign matches `w` |
| **PO-1.6** | operator closure (§1.6) | random inputs ⇒ every op output ∈ `[0,1]` |
| **PO-1.6b** | **all-back-edge node (§1.6)** | a node with in-edges but `in-deg_{G'}=0` and `:confidence=0.8` reports **0.8** (driver rule), not 0.5 |
| **PO-1.7** | incremental sweep (§1.7) | full re-sweep `==` cone re-sweep after a random `set_confidence!` |
| **PO-2.1** | Invariant L (§2.1) | random ops **incl. a term-drop on a surviving doc**; `postings ↔ forward` mutual inverses; scratch-rebuilt `==` incremental, compared as **canonicalized (sorted) sets** |
| **PO-2.2** | deletion (§2.2) | after random deletes, `df`,`n_docs`,postings `==` fresh rebuild |
| **PO-2.3** | exact cosine (§2.3) | `see_also` scores `==` brute-force dense cosine within `1e-5`; folds in `TermId` order |
| **PO-2.4** | pipeline determinism (§2.4) | shuffle **note *and* term insertion order** ⇒ identical `see_also/bm25/duplicates` **scores and order** |
| **PO-2.5** | dedup recall (§2.5) | blake3-exact dups: **all** returned; SimHash near-dups: recall **≥ threshold** over many seeded trials (not "all") |
| **PO-2.6** | **classify determinism (§7)** | permute term/centroid order ⇒ bit-identical Rocchio scores (same fold hazard as PO-1.4a) |
| **PO-3** | serde round-trip (§3) | **semantic** round-trip equality on every fixture (always); **byte**-identity only after canonical map serialization + regenerated goldens |
| **PO-3b** | **load-path rebuild (§3)** | `WasmNotebook::from_json` calls `rebuild_indices`; after a plain `from_str`, `index`/`in_edges`/`backlinks` are correct, not empty |
| **PO-4.1** | no double-borrow (§4.1) | a `combine` calling a mutator **directly** returns capability-denied, no panic under a live borrow |
| **PO-4.1b** | **nested-eval capability (§4.1)** | a `!` mutator reached via a higher-order builtin (`map`/`do`/`let`) under Reader context is still denied (capability monotonicity, I1) |
| **PO-4.2** | native≡λδ conformance (§4.2) | grid test over shipped ops within `1e-6` |
| **PO-4.3** | domain isolation (§4.3) | a foreign `:op`/`:type` is rejected; a mixed-scalar graph is unconstructable |
| **PO-5** | trigger termination (§5) | self-retriggering action terminates; bounded cascade respects depth `D` |
| **PO-5b** | **trigger order (§5)** | firing order is `triggers`-`Vec` order ⇒ final state deterministic for order-sensitive actions |
| **PO-6** | master determinism (§6) | end-to-end: shuffle **note + term + in-edge** insertion ⇒ identical index+conf+layout **bytes** (requires PO-1.4a, PO-2.4, PO-3 canonical) |

---

## 9. Handoff note to Opus

Implement in this order, each PR gated on its ledger rows:

1. **`index.rs` (headless).** Structure per §2.1 (doubly-linked postings — do **not** use sorted `Vec`s), lifecycle §2.2, exact query §2.3, determinism §2.4. Gate: PO-2.*, PO-3, PO-6(index part). No WASM, no UI. *This is the whole DEVONthink side de-risked before a pixel ships.*
2. **`edge.rs` + serde.** Additive `edges`; `in_edges` and `index` `#[serde(skip)]` (§3 ⚠). Gate: PO-3.
3. **`reason.rs`.** `ReasonGraph` (sorted, iterative DFS), `back_edges`, `kahn_topo`, `weight_edge`, native `combine`, `propagate`, cone cache. Gate: PO-1.*.
4. **λδ seam.** Reader-context `combine` dispatch (§4.1), conformance grid (§4.2), `validate` (§4.3). Gate: PO-4.*.
5. **`trigger.rs`.** Suppression guard (§5). Gate: PO-5.
6. **`layout.rs` + `ReasoningView`, See-Also/Duplicates panels.** Consume `back_edges` from `reason.rs` for dashed edges (§6 consistency). UI only after 1–5 are green.

**Blocking prerequisites (fix in the spec before step 1 — the v2 corrections):**
1. **Pin every float fold** in key-sorted order — `combine` in-edges by `(source NoteId, edge-idx)`; cosine numerator + both norms + classify centroids by `TermId` (`PO-1.4a`/`PO-2.4`/`PO-2.6`). Without this, `PO-6` is unachievable.
2. **Canonical map serialization** (`serialize_with` sorted, or `BTreeMap`) + regenerated goldens — *or* consciously relax `PO-3` to semantic equality (`PO-3`).
3. **Two-sided posting `remove`** so reindex preserves Invariant L (`PO-2.1`).
4. **Load path calls `rebuild_indices`** so derived state isn't empty after a plain deserialize (`PO-3b`).

These four are the difference between "compiles and passes" and "flakes on day one." Everything else is a caveat the ledger absorbs.

The math is settled above. Opus's job is faithful translation + turning the ledger green — not rediscovery.
