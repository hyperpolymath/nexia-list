// SPDX-License-Identifier: MPL-2.0
//! The λδ standard prelude: control-flow and threading sugar written **in λδ
//! itself** as hygienic macros (spec §3). Because macros are hygienic, the
//! temporaries these introduce (`v` in `and`/`or`, `tmp` in `if-let`) cannot
//! capture a user's identifiers — no `foo#` gensym ceremony required. This file
//! is the first real exercise of the hygiene machinery.

/// Evaluated once, at [`Interp::new`](super::Interp::new).
pub const PRELUDE: &str = r#"
;; --- conditionals -------------------------------------------------------

(defmacro when [test & body]
  `(if ~test (do ~@body) nil))

(defmacro unless [test & body]
  `(if ~test nil (do ~@body)))

(defmacro cond [& clauses]
  (if (empty? clauses)
    nil
    `(if ~(first clauses)
       ~(first (rest clauses))
       (cond ~@(rest (rest clauses))))))

;; --- short-circuit logic (temporaries are hygienic) ---------------------

(defmacro and [& xs]
  (if (empty? xs)
    true
    (if (empty? (rest xs))
      (first xs)
      `(let [v ~(first xs)] (if v (and ~@(rest xs)) v)))))

(defmacro or [& xs]
  (if (empty? xs)
    nil
    (if (empty? (rest xs))
      (first xs)
      `(let [v ~(first xs)] (if v v (or ~@(rest xs)))))))

;; --- threading ----------------------------------------------------------

(defmacro -> [x & forms]
  (if (empty? forms)
    x
    (let [form (first forms)
          threaded (if (list? form)
                     (cons (first form) (cons x (rest form)))
                     (list form x))]
      `(-> ~threaded ~@(rest forms)))))

(defmacro ->> [x & forms]
  (if (empty? forms)
    x
    (let [form (first forms)
          threaded (if (list? form)
                     (cons (first form) (concat (rest form) (list x)))
                     (list form x))]
      `(->> ~threaded ~@(rest forms)))))

;; --- binding conditionals (tmp is hygienic) -----------------------------

(defmacro if-let [binding then else]
  `(let [tmp ~(first (rest binding))]
     (if tmp
       (let [~(first binding) tmp] ~then)
       ~else)))

(defmacro when-let [binding & body]
  `(if-let ~binding (do ~@body) nil))
"#;
