// SPDX-License-Identifier: MPL-2.0
//! Kernel tests: reader round-trips, evaluator determinism, sandbox limits,
//! and a couple of property tests. All headless — no notebook, no host.
//!
//! Style note: assertions compare whole `Result`s (`assert_eq!(run(x), Ok(y))`)
//! rather than unwrapping. This asserts the *error variant* too, and keeps the
//! test module free of `unwrap()`/`panic!` — the production kernel is held
//! strictly panic-free, so nothing here masks a panic in a helper either.

use std::rc::Rc;

use super::{read_all, read_one, Budget, Interp, LdError, Value};

type Res = Result<Value, LdError>;

/// Evaluate `src` in a fresh interpreter under a generous budget.
fn run(src: &str) -> Res {
    Interp::new().eval_str(src, Budget::new())
}

// ── Reader ───────────────────────────────────────────────────────────────────

#[test]
fn reads_atoms() {
    assert_eq!(read_one("nil"), Ok(Value::Nil));
    assert_eq!(read_one("true"), Ok(Value::Bool(true)));
    assert_eq!(read_one("false"), Ok(Value::Bool(false)));
    assert_eq!(read_one("42"), Ok(Value::Int(42)));
    assert_eq!(read_one("-3"), Ok(Value::Int(-3)));
    assert_eq!(read_one("1.5"), Ok(Value::Float(1.5)));
    assert_eq!(read_one("1e3"), Ok(Value::Float(1000.0)));
    assert_eq!(read_one(":status"), Ok(Value::kw("status")));
    assert_eq!(read_one("title"), Ok(Value::sym("title")));
    assert_eq!(read_one("\"hi\\n\""), Ok(Value::str("hi\n")));
}

#[test]
fn nan_and_inf_are_symbols_not_numbers() {
    // The float parser must not swallow these identifiers.
    assert_eq!(read_one("nan"), Ok(Value::sym("nan")));
    assert_eq!(read_one("inf"), Ok(Value::sym("inf")));
    assert_eq!(read_one("->"), Ok(Value::sym("->")));
    assert_eq!(read_one("-"), Ok(Value::sym("-")));
}

#[test]
fn reads_collections() {
    assert_eq!(
        read_one("(+ 1 2)"),
        Ok(Value::list(vec![
            Value::sym("+"),
            Value::Int(1),
            Value::Int(2)
        ]))
    );
    assert_eq!(
        read_one("[1 2 3]"),
        Ok(Value::vector(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3)
        ]))
    );
    // Commas are whitespace.
    assert_eq!(read_one("[1, 2, 3]"), read_one("[1 2 3]"));
}

#[test]
fn reader_sugar_expands() {
    assert_eq!(
        read_one("'x"),
        Ok(Value::list(vec![Value::sym("quote"), Value::sym("x")]))
    );
    assert_eq!(
        read_one("`(a ~b ~@c)"),
        read_one("(quasiquote (a (unquote b) (unquote-splicing c)))")
    );
    // #(…) shorthand.
    assert_eq!(read_one("#(+ % 1)"), read_one("(fn [%1] (+ %1 1))"));
    assert_eq!(read_one("#(+ %1 %2)"), read_one("(fn [%1 %2] (+ %1 %2))"));
}

#[test]
fn reads_tagged_literals() {
    assert_eq!(
        read_one("#uuid \"abc\""),
        Ok(Value::Tagged {
            tag: Rc::from("uuid"),
            value: Rc::new(Value::str("abc")),
        })
    );
}

#[test]
fn line_comments_ignored() {
    assert_eq!(run("; a comment\n(+ 1 2) ; trailing"), Ok(Value::Int(3)));
}

#[test]
fn unbalanced_delimiters_error_not_panic() {
    assert!(read_all("(+ 1 2").is_err());
    assert!(read_all("[1 2}").is_err());
    assert!(read_all(")").is_err());
    assert!(read_all("\"unterminated").is_err());
}

// ── Arithmetic & comparison ─────────────────────────────────────────────────

#[test]
fn arithmetic() {
    assert_eq!(run("(+ 1 2 3)"), Ok(Value::Int(6)));
    assert_eq!(run("(- 10 3 2)"), Ok(Value::Int(5)));
    assert_eq!(run("(- 5)"), Ok(Value::Int(-5)));
    assert_eq!(run("(* 2 3 4)"), Ok(Value::Int(24)));
    assert_eq!(run("(/ 6 2)"), Ok(Value::Float(3.0))); // `/` always float
    assert_eq!(run("(mod 7 3)"), Ok(Value::Int(1)));
    assert_eq!(run("(+ 1 2.0)"), Ok(Value::Float(3.0))); // int+float promotes
    assert_eq!(run("(max 3 7 2)"), Ok(Value::Int(7)));
    assert_eq!(run("(floor 3.7)"), Ok(Value::Int(3)));
}

#[test]
fn equality_is_numeric_cross_type() {
    assert_eq!(run("(= 1 1.0)"), Ok(Value::Bool(true)));
    assert_eq!(run("(= 1 2)"), Ok(Value::Bool(false)));
    assert_eq!(run("(= \"a\" \"a\")"), Ok(Value::Bool(true)));
    assert_eq!(run("(= [1 2] [1 2])"), Ok(Value::Bool(true)));
    assert_eq!(run("(not= 1 2)"), Ok(Value::Bool(true)));
}

#[test]
fn comparison_chains() {
    assert_eq!(run("(< 1 2 3)"), Ok(Value::Bool(true)));
    assert_eq!(run("(< 1 3 2)"), Ok(Value::Bool(false)));
    assert_eq!(run("(>= 3 3 1)"), Ok(Value::Bool(true)));
}

#[test]
fn divide_by_zero_is_an_error() {
    assert!(matches!(run("(/ 1 0)"), Err(LdError::DivideByZero)));
    assert!(matches!(run("(mod 1 0)"), Err(LdError::DivideByZero)));
}

// ── Truthiness & special forms ──────────────────────────────────────────────

#[test]
fn truthiness() {
    assert_eq!(run("(if 0 :yes :no)"), Ok(Value::kw("yes"))); // 0 is truthy
    assert_eq!(run("(if \"\" :yes :no)"), Ok(Value::kw("yes"))); // "" is truthy
    assert_eq!(run("(if nil :yes :no)"), Ok(Value::kw("no")));
    assert_eq!(run("(if false :yes :no)"), Ok(Value::kw("no")));
    assert_eq!(run("(if true :yes)"), Ok(Value::kw("yes")));
    assert_eq!(run("(if false :yes)"), Ok(Value::Nil)); // no else → nil
}

#[test]
fn let_bindings_are_sequential() {
    assert_eq!(run("(let [x 1 y (+ x 1)] (* x y))"), Ok(Value::Int(2)));
}

#[test]
fn do_returns_last() {
    assert_eq!(run("(do 1 2 3)"), Ok(Value::Int(3)));
    assert_eq!(run("(do)"), Ok(Value::Nil));
}

#[test]
fn def_binds_globally() {
    let mut i = Interp::new();
    assert!(i.eval_str("(def x 41)", Budget::new()).is_ok());
    assert_eq!(i.eval_str("(+ x 1)", Budget::new()), Ok(Value::Int(42)));
}

#[test]
fn closures_and_recursion() {
    let prog = "(def fact (fn f [n] (if (<= n 1) 1 (* n (f (- n 1)))))) (fact 5)";
    assert_eq!(run(prog), Ok(Value::Int(120)));
}

#[test]
fn closures_capture_environment() {
    let prog = "(def make-adder (fn [n] (fn [x] (+ x n)))) (def add10 (make-adder 10)) (add10 5)";
    assert_eq!(run(prog), Ok(Value::Int(15)));
}

#[test]
fn variadic_rest_param() {
    assert_eq!(run("((fn [a & rest] rest) 1 2 3)"), run("[2 3]"));
    assert_eq!(run("((fn [a & rest] a) 1 2 3)"), Ok(Value::Int(1)));
}

// ── Higher-order & collections ──────────────────────────────────────────────

#[test]
fn higher_order_sequence_ops() {
    assert_eq!(run("(map #(* % %) [1 2 3])"), run("[1 4 9]"));
    assert_eq!(run("(filter #(> % 2) [1 2 3 4])"), run("[3 4]"));
    assert_eq!(run("(reduce + 0 [1 2 3 4])"), Ok(Value::Int(10)));
    assert_eq!(run("(reduce + [1 2 3 4])"), Ok(Value::Int(10)));
    assert_eq!(run("(count [1 2 3])"), Ok(Value::Int(3)));
    assert_eq!(run("(first [1 2 3])"), Ok(Value::Int(1)));
    assert_eq!(run("(rest [1 2 3])"), run("[2 3]"));
    assert_eq!(run("(reverse [1 2 3])"), run("[3 2 1]"));
    assert_eq!(run("(sort [3 1 2])"), run("[1 2 3]"));
    assert_eq!(run("(range 4)"), run("[0 1 2 3]"));
}

#[test]
fn sort_by_keyword_key() {
    assert_eq!(
        run("(sort-by :n [{:n 3} {:n 1} {:n 2}])"),
        run("[{:n 1} {:n 2} {:n 3}]")
    );
}

#[test]
fn keyword_as_function() {
    assert_eq!(run("(:status {:status \"todo\"})"), Ok(Value::str("todo")));
    assert_eq!(run("(:missing {:status \"todo\"})"), Ok(Value::Nil));
    assert_eq!(run("(:missing {:a 1} :default)"), Ok(Value::kw("default")));
    assert_eq!(run("(:a {:a 1} \"fallback\")"), Ok(Value::Int(1))); // present → not default
}

#[test]
fn maps_and_sets() {
    assert_eq!(run("(get {:a 1 :b 2} :b)"), Ok(Value::Int(2)));
    assert_eq!(run("(assoc {:a 1} :b 2)"), run("{:a 1 :b 2}"));
    assert_eq!(run("(dissoc {:a 1 :b 2} :a)"), run("{:b 2}"));
    assert_eq!(run("(keys {:a 1 :b 2})"), run("[:a :b]"));
    assert_eq!(run("(contains? #{:a :b} :a)"), Ok(Value::Bool(true)));
    assert_eq!(run("(union #{1 2} #{2 3})"), run("#{1 2 3}"));
    assert_eq!(run("(intersection #{1 2 3} #{2 3 4})"), run("#{2 3}"));
    assert_eq!(run("(count #{1 1 2})"), Ok(Value::Int(2))); // set dedups
}

#[test]
fn string_ops() {
    assert_eq!(run("(str \"a\" 1 :b)"), Ok(Value::str("a1:b")));
    assert_eq!(run("(upper \"hi\")"), Ok(Value::str("HI")));
    assert_eq!(run("(join \", \" [\"a\" \"b\"])"), Ok(Value::str("a, b")));
    assert_eq!(run("(count (words \"a b c\"))"), Ok(Value::Int(3)));
    assert_eq!(run("(includes? \"hello\" \"ell\")"), Ok(Value::Bool(true)));
    assert_eq!(run("(subs \"hello\" 1 3)"), Ok(Value::str("el")));
}

#[test]
fn collection_literals_evaluate_elements() {
    assert_eq!(run("[1 (+ 1 1) 3]"), run("[1 2 3]"));
    assert_eq!(run("{:sum (+ 1 2)}"), run("{:sum 3}"));
    // But quote keeps them literal.
    assert_eq!(
        run("'(+ 1 2)"),
        Ok(Value::list(vec![
            Value::sym("+"),
            Value::Int(1),
            Value::Int(2)
        ]))
    );
}

// ── Quasiquote ──────────────────────────────────────────────────────────────

#[test]
fn quasiquote_unquote_and_splice() {
    // Sequential equality treats the produced list as equal to the same items.
    assert_eq!(
        run("(let [x 2 xs [3 4]] `(1 ~x ~@xs 5))"),
        run("(list 1 2 3 4 5)")
    );
}

// ── Reflection ──────────────────────────────────────────────────────────────

#[test]
fn eval_and_read_reflection() {
    assert_eq!(run("(eval '(+ 1 2))"), Ok(Value::Int(3)));
    assert_eq!(run("(read \"(+ 1 2)\")"), run("'(+ 1 2)"));
    assert_eq!(run("(type 5)"), Ok(Value::kw("int")));
    assert_eq!(run("(type :x)"), Ok(Value::kw("keyword")));
}

// ── Errors as values ────────────────────────────────────────────────────────

#[test]
fn errors_are_structured_not_panics() {
    assert!(matches!(run("undefined-symbol"), Err(LdError::Unbound(_))));
    assert!(matches!(run("(1 2 3)"), Err(LdError::NotCallable(_))));
    assert!(matches!(run("(+ 1 \"a\")"), Err(LdError::Type { .. })));
    assert!(matches!(run("(if)"), Err(LdError::Syntax { .. })));
}

// ── Sandbox budget ──────────────────────────────────────────────────────────

#[test]
fn recursion_depth_budget_aborts() {
    let mut i = Interp::new();
    let r = i.eval_str(
        "(def spin (fn [] (spin))) (spin)",
        Budget::with_limits(1_000_000, 128),
    );
    assert!(matches!(r, Err(LdError::Budget(_))), "got {r:?}");
}

#[test]
fn step_budget_aborts() {
    let mut i = Interp::new();
    let r = i.eval_str("(reduce + (range 1000))", Budget::with_limits(50, 64));
    assert!(matches!(r, Err(LdError::Budget(_))), "got {r:?}");
}

#[test]
fn determinism_same_input_same_output() {
    let prog = "(sort-by :p (map #(assoc % :p (* 2 (:n %))) [{:n 3} {:n 1} {:n 2}]))";
    assert_eq!(run(prog), run(prog));
}

// ── Macros, hygiene & multimethods ──────────────────────────────────────────

#[test]
fn user_defmacro_expands_and_evaluates() {
    assert_eq!(
        run("(defmacro my-when [c e] `(if ~c ~e nil)) (my-when true 42)"),
        Ok(Value::Int(42))
    );
    assert_eq!(
        run("(defmacro my-when [c e] `(if ~c ~e nil)) (my-when false 42)"),
        Ok(Value::Nil)
    );
}

#[test]
fn macroexpand_reveals_the_expansion() {
    // Marks are internal; `str` prints the base name, so the head reads as `if`.
    assert_eq!(
        run("(str (first (macroexpand-1 '(when true 1))))"),
        Ok(Value::str("if"))
    );
}

#[test]
fn prelude_conditionals() {
    assert_eq!(run("(when true 1 2 3)"), Ok(Value::Int(3)));
    assert_eq!(run("(when false 1)"), Ok(Value::Nil));
    assert_eq!(run("(unless false 7)"), Ok(Value::Int(7)));
    assert_eq!(run("(cond false 1 true 2 :else 3)"), Ok(Value::Int(2)));
    assert_eq!(run("(cond false 1 false 2)"), Ok(Value::Nil));
}

#[test]
fn prelude_short_circuit_logic() {
    assert_eq!(run("(and)"), Ok(Value::Bool(true)));
    assert_eq!(run("(and 1 2 3)"), Ok(Value::Int(3)));
    assert_eq!(run("(and 1 nil 3)"), Ok(Value::Nil));
    assert_eq!(run("(or)"), Ok(Value::Nil));
    assert_eq!(run("(or nil false 5)"), Ok(Value::Int(5)));
    assert_eq!(run("(or nil false)"), Ok(Value::Bool(false)));
}

#[test]
fn prelude_threading() {
    assert_eq!(run("(-> 5 (+ 1) (* 2))"), Ok(Value::Int(12)));
    // Non-commutative op distinguishes -> (arg first) from ->> (arg last).
    assert_eq!(run("(-> 10 (- 3))"), Ok(Value::Int(7)));
    assert_eq!(run("(->> 10 (- 3))"), Ok(Value::Int(-7)));
    // Bare-symbol steps thread too.
    assert_eq!(run("(-> [3 1 2] sort first)"), Ok(Value::Int(1)));
}

#[test]
fn prelude_if_let() {
    assert_eq!(run("(if-let [x 5] (* x x) :none)"), Ok(Value::Int(25)));
    assert_eq!(run("(if-let [x nil] x :none)"), Ok(Value::kw("none")));
    assert_eq!(run("(when-let [x 3] (+ x 1))"), Ok(Value::Int(4)));
}

#[test]
fn hygiene_introduced_binding_never_captures() {
    // The macro introduces `x`; the caller also has `x`. The caller's `x`
    // spliced via `~e` must NOT be captured by the macro's `x = 2`.
    let prog = "(defmacro m [e] `(let [x 2] ~e)) (let [x 1] (m x))";
    assert_eq!(run(prog), Ok(Value::Int(1)));
}

#[test]
fn hygiene_free_identifier_is_referentially_transparent() {
    // The macro's `+` must mean the global `+`, even though the caller has
    // locally rebound `+` to `-`.
    let prog = "(defmacro inc [n] `(+ ~n 1)) (let [+ -] (inc 5))";
    assert_eq!(run(prog), Ok(Value::Int(6)));
}

#[test]
fn prelude_temporaries_are_hygienic() {
    // `and`'s internal temp must not capture a user `v`.
    assert_eq!(run("(let [v 10] (and true v))"), Ok(Value::Int(10)));
}

#[test]
fn gensym_is_fresh() {
    assert_eq!(run("(= (gensym) (gensym))"), Ok(Value::Bool(false)));
    assert_eq!(
        run("(includes? (str (gensym \"foo\")) \"foo__\")"),
        Ok(Value::Bool(true))
    );
}

#[test]
fn multimethods_dispatch_on_type() {
    let prog = r##"
        (defmulti describe :type)
        (defmethod describe "task" [n] (str "task:" (:title n)))
        (defmethod describe :default [n] "other")
        [(describe {:type "task" :title "T"}) (describe {:type "note"})]
    "##;
    assert_eq!(
        run(prog),
        Ok(Value::vector(vec![
            Value::str("task:T"),
            Value::str("other")
        ]))
    );
}

// ── Property tests ──────────────────────────────────────────────────────────

use proptest::prelude::*;

/// A generator for round-trippable values (no floats — their textual form is
/// not exactly round-trippable in general; covered by unit tests instead).
fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Nil),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Int),
        "[a-z][a-z0-9-]{0,6}"
            .prop_filter("not a literal", |s| !matches!(
                s.as_str(),
                "nil" | "true" | "false"
            ))
            .prop_map(Value::sym),
        "[a-z][a-z0-9-]{0,6}".prop_map(Value::kw),
        "[a-zA-Z0-9 ]{0,8}".prop_map(Value::str),
    ];
    leaf.prop_recursive(4, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::list),
            prop::collection::vec(inner, 0..4).prop_map(Value::vector),
        ]
    })
}

/// Pure, bounded expressions used to check determinism across independent
/// interpreters. Small integer leaves and a shallow tree avoid making machine
/// overflow part of the property under test.
fn arb_pure_expr() -> impl Strategy<Value = String> {
    (-10i64..=10)
        .prop_map(|n| n.to_string())
        .prop_recursive(3, 32, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("(+ {a} {b})")),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("(- {a} {b})")),
                (inner.clone(), inner).prop_map(|(a, b)| format!("(* {a} {b})")),
            ]
        })
}

fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,6}".prop_filter("must parse as a symbol", |name| {
        !matches!(name.as_str(), "nil" | "true" | "false")
    })
}

proptest! {
    /// The reader never panics on arbitrary input (it may Err, but must not crash).
    #[test]
    fn reader_never_panics(s in any::<String>()) {
        let _ = read_all(&s);
    }

    /// Printing a value then reading it back yields an equal value.
    #[test]
    fn print_read_round_trip(v in arb_value()) {
        let printed = v.to_string();
        prop_assert_eq!(read_one(&printed), Ok(v));
    }

    /// A pure expression has the same result in independent fresh evaluators.
    #[test]
    fn pure_evaluation_is_deterministic(src in arb_pure_expr()) {
        let left = Interp::new().eval_str(&src, Budget::with_limits(10_000, 64));
        let right = Interp::new().eval_str(&src, Budget::with_limits(10_000, 64));
        prop_assert_eq!(left, right);
    }

    /// A binding introduced by a macro cannot capture a caller binding,
    /// including when both happen to use exactly the same printed identifier.
    #[test]
    fn macro_introduced_binding_does_not_capture(
        introduced in arb_identifier(),
        caller in arb_identifier(),
    ) {
        let src = format!(
            "(defmacro proof-m [e] `(let [{introduced} 2] ~e)) \
             (let [{caller} 1] (proof-m {caller}))"
        );
        prop_assert_eq!(
            Interp::new().eval_str(&src, Budget::with_limits(10_000, 64)),
            Ok(Value::Int(1)),
        );
    }
}
