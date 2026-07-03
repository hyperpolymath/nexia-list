<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# LambdaDelta (λδ) — Language specification, v0 DRAFT

> *A note is a letter we send to our future self.* — *The Tinderbox Way*
>
> λδ exists to enlarge that correspondence. This document pins the three
> fundamentals we agreed to nail before writing the interpreter (see
> [ADR-0003](../adr/0003-lambdadelta-lisp-substrate.md)): **surface syntax**,
> the **note-as-value model**, and the **initial builtin vocabulary**.

**Status:** DRAFT — several load-bearing choices are marked **[DECISION]** and
need your confirmation before implementation. Recommended defaults are shown;
nothing here is built yet.

---

## 0. Stance

λδ is a *successor-flavoured* Lisp, not a museum piece: homoiconic and
macro-capable like Scheme, but with modern, readable data literals (vectors,
maps, keywords) in the spirit of Clojure — because the first people to meet λδ
are Tinderbox users writing a one-line formula, not Lisp hackers. Fewer quotes,
less ceremony, same power underneath.

Everything is an expression that returns a value. Effects on the notebook are
explicit, named with a trailing `!`, and only permitted in contexts that allow
them.

---

## 1. Surface syntax  **[DECISION 1: flavour]**

**Recommended: Clojure-flavoured.** (Alternative: Scheme-classic — `()` for
both code and lists, `,`/`,@` for unquote, `#t`/`#f`. I recommend against it for
the L1 audience.)

### Literals
```
nil                      ; absence / empty
true  false              ; booleans
42   -3   1.5   1e9      ; numbers (i64 or f64; see §2)
"hello\n"                ; strings (JSON-style escapes)
:status  :due-date       ; keywords (self-evaluating, interned; used as map keys)
title  ->md  note?       ; symbols (kebab-case; ? = predicate, ! = mutator)
```

### Collections
```
(f a b)                  ; a call: apply f to a, b        — LIST, also code
[1 2 3]                  ; a vector (indexed data)         [DECISION 4]
{:k 1 :j 2}              ; a map (keyword→value)           [DECISION 3]
```

### Reader sugar
```
'x        => (quote x)
`x        => (quasiquote x)
~x        => (unquote x)
~@xs      => (unquote-splicing xs)
;; a comment to end of line
```

That is the whole surface. Code is data: `(+ 1 2)` is a three-element list whose
head is the symbol `+`. Macros manipulate exactly these forms.

---

## 2. Value model  **[DECISION 2: note representation]**

Runtime values:

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
| `map` | keyword→value (string keys also accepted on read) |
| `function` | closure (builtin or user `fn`) |

### Truthiness  **[DECISION 5]**
**Recommended (Clojure-style):** only `false` and `nil` are falsy; everything
else — including `0` and `""` — is truthy.

### A note *is a map* (recommended)
Reading a note yields an **immutable snapshot map** — a note is plain data, the
most Lisp-y choice and the one that makes the notebook homoiconic:

```clojure
{:id         "11111111-1111-4111-8111-111111111111"
 :title      "Meeting notes"
 :content    "…discussed the roadmap and [[Beta]]…"
 :attrs      {:status "todo" :priority 2}
 :links      ["2222…"]           ; outgoing note ids
 :backlinks  ["3333…"]           ; incoming note ids (read-only)
 :position   [120.0 80.0]        ; or nil if unplaced
 :size       [200.0 150.0]       ; or nil
 :prototype  nil                 ; or a note id
 :created-at "2026-01-01T00:00:00Z"
 :modified-at "2026-01-02T00:00:00Z"}
```

- In a **formula** or **action**, the symbol `self` is bound to this map for the
  current note. `(attr self :status)` → `"todo"`.
- **Reading is pure**: a note map is a value, not a live handle; it never
  changes under you.
- **Mutation is explicit and id-based**: `(set-attr! (:id self) :status "done")`.
  Mutators take an id (or a note map, from which the id is read) and return the
  updated note map (or a delta), so effects are visible and testable. *(Alt
  considered and rejected for v0: opaque live note handles — less homoiconic,
  harder to sandbox and test.)*

### Attribute ↔ JSON mapping
Notebook attributes are stored as JSON; the bridge is total and lossless:

| JSON | λδ |
|---|---|
| string | string |
| number | number |
| `true`/`false` | bool |
| `null` | `nil` |
| array | **vector** |
| object | **map** (string keys read as keywords; see [DECISION 3]) |

---

## 3. Special forms

The irreducible core the evaluator knows directly (everything else is a
function or a macro):

```clojure
(quote x)                      ; unevaluated x                 '  sugar
(if test then else?)           ; else defaults to nil
(do e1 e2 … en)                ; sequence; value is en
(let [x 1  y (+ x 1)] body…)   ; sequential local bindings
(fn [a b] body…)               ; lambda / closure
(fn name [a b] body…)          ; self-referential lambda
(def name value)               ; define in the notebook's environment
(defmacro name [args] body…)   ; compile-time expansion
(quasiquote t) / (unquote e) / (unquote-splicing e)   ; ` ~ ~@
```

Provided as macros over the above (still "core" to users):
`cond`, `when`, `and`, `or`, `->` / `->>` (threading), `if-let`, `case`.

---

## 4. Initial builtin vocabulary

A deliberately small, growable standard library. `?` = predicate, `!` = mutator.

**Arithmetic / compare / logic**
`+ - * / mod` · `= not= < > <= >=` · `not min max abs floor ceil round`

**Predicates**
`nil? true? false? number? string? symbol? keyword? list? vector? map? fn? note? empty?`

**Sequences** (work on lists and vectors)
`list vector count first rest last nth get take drop reverse sort sort-by
range conj cons concat map filter remove reduce some every? distinct
into flatten`

**Strings**
`str join split lines words trim lower upper starts-with? ends-with?
includes? replace subs format`

**Maps**
`get assoc dissoc update keys vals contains? merge select-keys`

**Notebook — readers (pure)**
```clojure
(notes)                 ; vector of all note maps
(note id)               ; the note map, or nil
(title n) (content n) (attrs n) (links n) (backlinks n) (position n)
(attr n key)            ; one attribute value, or nil
(search q)              ; vector of note maps matching a text query
(agents)                ; vector of agent maps {:id :name :query}
(run-agent id)          ; vector of note maps the agent collects
(resolve-title s)       ; note id whose title = s (case-insensitive), or nil
```

**Notebook — mutators (only in action contexts)**
```clojure
(create-note! title)              ; -> new note map
(create-note! title x y)          ; placed on the canvas
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
  file/DOM access. The only effects are the `!` notebook mutators, only in
  action contexts.
- **Bounded:** every evaluation runs under a *budget* — a maximum reduction-step
  count, a wall-clock ceiling, and a recursion-depth limit. Exceeding it aborts
  cleanly with an error value; it never hangs the tab. (Heavy jobs move to a Web
  Worker in a later phase.)
- **Errors are values / diagnostics**, never panics: unbound symbol, arity
  mismatch, type error, budget-exceeded — each yields a structured error the UI
  can show against the offending form.

---

## 7. Where λδ code lives (persistence)

Homoiconicity in practice: user definitions are **data in the notebook**, so
they save/load/sync with everything else and a notebook carries its own
behaviour.

- Computed-attribute **formulas** and agent **queries/actions** live on the note
  / agent that owns them (as source strings + parsed forms).
- Shared **functions and macros** live in a notebook-level `:lambdadelta`
  section (a map of name → source).
- **[DECISION 6]** A textual interchange form for sharing packages between
  notebooks — recommended extension **`.ld`** (plain λδ source) — so power users
  can publish/import a package as a file. (The in-notebook form is the source of
  truth; `.ld` is for exchange.)

---

## 8. Worked examples (the ladder, in practice)

```clojure
;; L1 — a computed attribute "wordcount" (pure formula, self = this note)
(count (words (content self)))

;; L1 — an agent predicate: notes due and not done
(and (= (attr self :status) "todo")
     (attr self :due-date))

;; L2 — an on-create action: stamp new notes from a prototype
(do (set-attr! (:id self) :status "inbox")
    (set-attr! (:id self) :created-day (subs (:created-at self) 0 10)))

;; L2 — a macro: define a query shorthand `(tagged :x)`
(defmacro tagged [t]
  `(filter #(includes? (or (attr % :tags) []) ~t) (notes)))

;; L3 — resurfacing: last 5 notes I touched that link to "Project X"
(->> (notes)
     (filter #(includes? (links %) (resolve-title "Project X")))
     (sort-by :modified-at)
     reverse
     (take 5))

;; L4 — a computational note: content evaluates, result renders inline
;; (a table of open tasks by priority) — same language, richer surface.
```

---

## 9. Open decisions to confirm  (the forks worth agreeing before code)

1. **Syntax flavour** — *Recommend Clojure-flavoured* (`[]` vectors, `{}` maps,
   `:keywords`, `~` unquote, `true/false/nil`) over Scheme-classic. Modern,
   readable, less quoting for the L1 audience.
2. **Note representation** — *Recommend note = immutable snapshot map*; mutation
   via id-based `!` builtins. (vs opaque live handles.)
3. **Map keys** — *Recommend keywords canonical* (`:status`), with string keys
   accepted on read so JSON round-trips cleanly. `(attr n :status)` and
   `(attr n "status")` both work.
4. **Data sequence default** — *Recommend vectors `[]` for data*, lists `()` for
   code (Clojure split), rather than lists-for-everything.
5. **Truthiness** — *Recommend only `nil` and `false` are falsy* (so `0`/`""`
   are truthy), matching modern Lisps and JSON `null → nil`.
6. **Package interchange** — *Recommend a `.ld` text form* for sharing, with the
   in-notebook data form as source of truth.
7. **Name/typography** — keep **LambdaDelta / λδ**; the ASCII trigraph in code
   and identifiers is `ld` (module `core/src/lambdadelta/`, files `*.ld`).

### Successor-language advances to fold in (30 years of Lisp evolution)

These pull modern Lisp evolution into λδ rather than shipping "1994 with
parentheses." All recommended; flagged so we agree the language's shape before
the interpreter is written.

8. **Macro hygiene** — *Recommend hygienic-by-default* (`syntax-rules`/
   `syntax-case`-style, or gensym-enforced) rather than capture-prone classic
   `defmacro`. For non-expert users, macros that can silently capture a variable
   are a footgun; this is the most important single "successor" lesson.
9. **Extensibility via multimethods / protocols** — *Recommend* open
   multimethods (dispatch on any function of the args) and protocols, so users
   can extend behaviour **by note type / prototype** — e.g. how a `:type`
   renders, exports, or reacts. This is the lever that makes Nexia-List
   *moldable* (Emacs-like) while staying simple by default. (Not class-based
   OO/CLOS — data-orientation + dispatch, per Clojure's turn away from classes.)
10. **Sets `#{}`** — *Recommend* set literals + set ops; Tinderbox attributes are
    frequently set-valued (tags), so first-class sets pay off immediately.
11. **Pattern matching `match`** — *Recommend* a `match` macro for destructuring
    notes/attributes/results; the standard modern ergonomic.
12. **Tagged literals `#uuid` / `#inst`** — *Recommend* reader tags so note ids
    and dates are natural literals, extensible for future types.

Deliberately **skipped** for now (overkill for a note tool): delimited
continuations, STM/`core.async` concurrency, transducers, full gradual typing.
`spec`-style attribute schemas are deferred to the typed-attributes work.

Confirm/redirect §1–§12 and the interpreter (Phase L0) can be built directly
against this spec.
