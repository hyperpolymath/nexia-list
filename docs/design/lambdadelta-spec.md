<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# LambdaDelta (λδ) — Language specification, v0.1

> *A note is a letter we send to our future self.* — *The Tinderbox Way*
>
> λδ exists to enlarge that correspondence. This document pins the fundamentals
> we agreed before writing the interpreter (see
> [ADR-0003](../adr/0003-lambdadelta-lisp-substrate.md)): **surface syntax**,
> the **note-as-value model**, the **builtin vocabulary**, and the **kernel/host
> seam** that makes an SDK/plugin ecosystem cheap.

**Status:** v0.1 — the load-bearing decisions are **confirmed** (see §9).
Package format, capability model, and the dev wizard are tracked separately in
**[issue #33](https://github.com/hyperpolymath/nexia-list/issues/33)**. No code
yet; this is the target Phase L0 builds against.

---

## 0. Stance

λδ is a *successor-flavoured* Lisp, not a museum piece: homoiconic and
macro-capable like Scheme, with **hygienic macros** and modern, readable data
literals (vectors, maps, sets, keywords) in the spirit of Clojure — because the
first people to meet λδ are Tinderbox users writing a one-line formula, not Lisp
hackers. Fewer quotes, less ceremony, same power underneath.

Everything is an expression that returns a value. Effects on the notebook are
explicit, named with a trailing `!`, and only permitted in contexts that allow
them.

---

## 1. Surface syntax  *(Clojure-flavoured — confirmed)*

### Literals
```
nil                      ; absence / empty
true  false              ; booleans
42   -3   1.5   1e9      ; numbers (i64 or f64; see §2)
"hello\n"                ; strings (JSON-style escapes)
:status  :due-date       ; keywords (self-evaluating, interned; map keys)
title  ->md  note?       ; symbols (kebab-case; ? = predicate, ! = mutator)
#uuid "1111…"            ; tagged literal — a note id
#inst "2026-01-01T…"     ; tagged literal — an instant
```

### Collections
```
(f a b)                  ; a call: apply f to a, b        — LIST, also code
[1 2 3]                  ; a vector (indexed data)
{:k 1 :j 2}              ; a map (keyword→value)
#{:a :b}                 ; a set (Tinderbox attributes are often set-valued)
```

### Reader sugar
```
'x        => (quote x)
`x        => (quasiquote x)
~x        => (unquote x)
~@xs      => (unquote-splicing xs)
#(… % …)  => (fn [%] (… % …))        ; anonymous-fn shorthand
#tag v    => a tagged literal (extensible; #uuid, #inst built in)
;; a comment to end of line
```

That is the whole surface. Code is data: `(+ 1 2)` is a three-element list whose
head is the symbol `+`. Macros manipulate exactly these forms.

---

## 2. Value model  *(note = immutable map — confirmed)*

| λδ value | Notes |
|---|---|
| `nil` | absence; the only "empty" |
| `bool` | `true` / `false` |
| `number` | integer (i64) or float (f64); `/` and decimals produce floats |
| `string` | UTF-8 |
| `symbol` | identifiers (evaluated: looked up in scope) |
| `keyword` | `:like-this`; self-evaluating; canonical map key |
| `list` | linked sequence; the form of code |
| `vector` | indexed sequence; the form of data |
| `set` | `#{…}`; unordered, unique members |
| `map` | keyword→value (string keys also accepted on read) |
| `function` | closure (builtin or user `fn`) |

### Truthiness  *(only `nil` and `false` are falsy — confirmed)*
Everything else — including `0` and `""` — is truthy.

### A note *is a map*
Reading a note yields an **immutable snapshot map** — a note is plain data, the
most Lisp-y choice and the one that makes the notebook homoiconic:

```clojure
{:id         #uuid "1111…"
 :title      "Meeting notes"
 :content    "…discussed the roadmap and [[Beta]]…"
 :attrs      {:status "todo" :priority 2 :tags #{:work :q3}}
 :links      [#uuid "2222…"]      ; outgoing note ids
 :backlinks  [#uuid "3333…"]      ; incoming note ids (read-only)
 :position   [120.0 80.0]         ; or nil if unplaced
 :size       [200.0 150.0]        ; or nil
 :prototype  nil                  ; or a note id
 :type       nil                  ; user-facing dispatch tag (see §3, multimethods)
 :created-at #inst "2026-01-01T00:00:00Z"
 :modified-at #inst "2026-01-02T00:00:00Z"}
```

- In a **formula** or **action**, the symbol `self` is bound to this map.
  `(attr self :status)` → `"todo"`.
- **Reading is pure**: a note map is a value, not a live handle; it never
  changes under you.
- **Mutation is explicit and id-based**: `(set-attr! (:id self) :status "done")`.
  Mutators take an id (or a note map, from which the id is read) and return the
  updated note map (or a delta), so effects are visible and testable.

### Attribute ↔ JSON mapping (total, lossless)
string→string · number→number · bool→bool · `null`→`nil` · array→**vector** ·
object→**map** (string keys read as keywords). *(Sets serialize as a tagged
array `#{…}` so JSON stays canonical; typed-attribute work later may make set
membership explicit.)*

---

## 3. Special forms & extensibility

The irreducible core the evaluator knows directly (everything else is a function
or a macro):

```clojure
(quote x)                      ; unevaluated x                 '  sugar
(if test then else?)           ; else defaults to nil
(do e1 … en)                   ; sequence; value is en
(let [x 1  y (+ x 1)] body…)   ; sequential bindings; supports destructuring
(fn [a b] body…)               ; lambda / closure (also (fn name […] …))
(def name value)               ; define in the notebook environment
(defmacro name [args] body…)   ; HYGIENIC by default (confirmed)
(quasiquote t) / (unquote e) / (unquote-splicing e)   ; ` ~ ~@
```

**Hygienic macros (confirmed).** Macro-introduced bindings never capture, and
references never leak, without the author asking — `syntax-rules`/`syntax-case`
-grade hygiene, `gensym` available for deliberate cases. Non-expert authors get
safe macros for free; an explicit unhygienic escape hatch may come later if
justified.

**Multimethods / protocols (confirmed) — the extensibility lever.** Open
dispatch on *any* function of the arguments (not class-based OO), so plugins and
prototypes can extend behaviour **by note `:type`**:

```clojure
(defmulti render :type)                      ; dispatch on a note's :type
(defmethod render "task"  [n] (task-card n))
(defmethod render :default [n] (plain-card n))
```

This is what makes Nexia-List *moldable* (Emacs-like) while staying simple by
default: a plugin adds `defmethod`s for its note types without touching anyone
else's code.

Provided as (hygienic) macros over the above — still "core" to users:
`cond`, `when`, `and`, `or`, `->` / `->>` (threading), `if-let`, `case`,
`match` (pattern-match/destructure notes, attrs, results).

---

## 4. Initial builtin vocabulary

A deliberately small, growable standard library. `?` = predicate, `!` = mutator.

**Arithmetic / compare / logic**
`+ - * / mod` · `= not= < > <= >=` · `not min max abs floor ceil round`

**Predicates**
`nil? true? false? number? string? symbol? keyword? list? vector? set? map? fn? note? empty?`

**Sequences** (lists and vectors)
`list vector count first rest last nth get take drop reverse sort sort-by range
conj cons concat map filter remove reduce some every? distinct into flatten`

**Sets**
`set union intersection difference subset? contains? conj disj`

**Strings**
`str join split lines words trim lower upper starts-with? ends-with? includes?
replace subs format`

**Maps**
`get assoc dissoc update keys vals contains? merge select-keys`

**Notebook — readers (pure)**
```clojure
(notes) (note id)
(title n) (content n) (attrs n) (links n) (backlinks n) (position n)
(attr n key)            ; one attribute value, or nil
(search q)              ; vector of note maps matching a text query
(agents) (run-agent id)
(resolve-title s)       ; note id whose title = s (case-insensitive), or nil
```

**Notebook — mutators (only in action contexts)**
```clojure
(create-note! title) (create-note! title x y)
(set-title! id s) (set-content! id s)
(set-attr! id key v) (remove-attr! id key)
(move-note! id x y) (resize-note! id w h)
(link! from to) (unlink! from to)
(delete-note! id)
```

**Reflection (homoiconicity)**
`eval read quote gensym macroexpand`

---

## 5. Evaluation contexts

Same language, different capabilities and bound variables — this is how one
engine serves every surface without leaking parentheses into L0:

| Context | Bound vars | May mutate? | Must return | Example surface |
|---|---|---|---|---|
| **Expression** | — | no | any value | the REPL |
| **Formula** (computed attribute) | `self` | no (pure) | attribute value | `(count (words (content self)))` |
| **Agent predicate** | `self` | no | truthy | `(= (attr self :status) "todo")` |
| **Agent query** | — | no | vector of notes/ids | `(filter #(> (attr % :priority) 3) (notes))` |
| **Action** (on-create / agent-action / stamp / adornment) | `self` | **yes** | ignored | `(set-attr! (:id self) :seen true)` |

The existing L0 agent DSL (`attr:status=todo`) compiles to an **Agent predicate**
λδ expression — one engine, two surfaces.

---

## 6. Sandbox contract

- **Deterministic & pure-by-default:** no clock, no randomness, no network, no
  file/DOM access. The only effects are the `!` notebook mutators, in action
  contexts, subject to granted capabilities (§7).
- **Bounded:** every evaluation runs under a *budget* — a reduction-step count,
  a wall-clock ceiling, and a recursion-depth limit. Exceeding it aborts cleanly
  with an error value; it never hangs the tab. (Heavy jobs → Web Worker later.)
- **Errors are values / diagnostics**, never panics: unbound symbol, arity
  mismatch, type error, capability-denied, budget-exceeded — each a structured
  error the UI can show against the offending form.

---

## 7. Architecture: the kernel / host seam  *(confirmed — the SDK enabler)*

The single discipline that makes an SDK, embedding, and the plugin ecosystem
(#33) cheap rather than a later rewrite. Build λδ in two layers from day one:

- **Kernel** (`core/src/lambdadelta/`) — reader · value model · evaluator ·
  hygienic macros · multimethods · budget. **Knows nothing about notes.** A
  self-contained language that could become its own crate.
- **Host bindings** — the notebook builtins (`notes`, `set-attr!`, `render`, …)
  are *registered into* the kernel through a host-function interface, each
  carrying the **capability** it requires. Nexia-List is simply the *first host*.

Consequences that fall out for free:
- **Embedders** depend on the kernel + `register_builtin`; λδ becomes an
  ecosystem beyond Nexia-List.
- **Plugin authors** ship a package (`.ld` source + a manifest of entry points
  and *requested capabilities*); the provisioner/configurator (#33) grant and
  enforce them; the harness runs them against a fixture notebook under the
  budget.
- **Sugar is free**: almost all developer sweetness (`defcommand`, `defview`,
  `deftemplate`, threading, `match`, destructuring) is hygienic macros + reader
  tags *around* the kernel — the kernel stays tiny and stable.

The capability model and package/manifest format are specified in **#33**; this
section fixes only that the seam exists and where it sits.

---

## 8. Where λδ code lives (persistence) & worked examples

User definitions are **data in the notebook**, so they save/load/sync with
everything and a notebook carries its own behaviour. Formulas and agent
queries/actions live on their owning note/agent; shared functions, macros, and
`defmethod`s live in a notebook-level `:lambdadelta` map (name → source).
A textual **`.ld`** form is the interchange unit for sharing packages between
notebooks (the in-notebook data form is the source of truth).

```clojure
;; L1 — computed attribute "wordcount" (pure; self = this note)
(count (words (content self)))

;; L1 — agent predicate: due and not done
(and (= (attr self :status) "todo") (attr self :due-date))

;; L2 — on-create action: stamp a new note
(do (set-attr! (:id self) :status "inbox")
    (set-attr! (:id self) :created-day (subs (str (:created-at self)) 0 10)))

;; L2 — a hygienic macro: query shorthand (tagged :work)
(defmacro tagged [t]
  `(filter #(contains? (or (attr % :tags) #{}) ~t) (notes)))

;; L2 — extend rendering for a note type (plugin-style)
(defmethod render "meeting" [n] (meeting-card n))

;; L3 — resurfacing: last 5 notes I touched that link to "Project X"
(->> (notes)
     (filter #(includes? (links %) (resolve-title "Project X")))
     (sort-by :modified-at) reverse (take 5))

;; L4 — a computational note: content evaluates, result renders inline.
```

---

## 9. Decisions (v0.1 — confirmed)

1. **Flavour** — Clojure-flavoured (`[] {} #{} :kw`, `~` unquote, `true/false/nil`).
2. **Note representation** — immutable snapshot map; mutation via id-based `!`.
3. **Map keys** — keywords canonical; string keys accepted on read.
4. **Data sequences** — vectors `[]` for data, lists `()` for code.
5. **Truthiness** — only `nil` and `false` are falsy.
6. **Macros** — **hygienic by default**.
7. **Extensibility** — **multimethods + protocols** (dispatch on any fn; not class-OO).
8. **Sugar** — sets `#{}`, threading `-> ->>`, `match`, tagged literals `#uuid`/`#inst`, `#(…)`.
9. **Architecture** — **kernel/host seam**: notebook-agnostic kernel + registered host bindings (§7).
10. **Interchange** — `.ld` text form for package sharing.
11. **Name/typography** — **LambdaDelta / λδ**; ASCII trigraph `ld`; module `core/src/lambdadelta/`.

**Deferred (tracked in [#33](https://github.com/hyperpolymath/nexia-list/issues/33)):**
package/manifest format, the capability model, and the minter/provisioner/
configurator/harness dev wizard.

**Deliberately skipped for now** (overkill for a note tool): delimited
continuations, STM/`core.async` concurrency, transducers, full gradual typing.
`spec`-style attribute schemas are deferred to the typed-attributes work.
