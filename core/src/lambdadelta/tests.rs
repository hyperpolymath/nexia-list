// SPDX-License-Identifier: MPL-2.0
//! Kernel tests: reader round-trips, evaluator determinism, sandbox limits,
//! and a couple of property tests. All headless — no notebook, no host.

use super::{read_all, read_one, Budget, Interp, LdError, Value};

/// Evaluate `src` in a fresh interpreter under a generous budget, expecting Ok.
fn eval(src: &str) -> Value {
    Interp::new()
        .eval_str(src, Budget::new())
        .unwrap_or_else(|e| panic!("eval of {src:?} failed: {e}"))
}

/// Evaluate `src`, returning the raw result (for error-path assertions).
fn try_eval(src: &str) -> Result<Value, LdError> {
    Interp::new().eval_str(src, Budget::new())
}

// ── Reader ───────────────────────────────────────────────────────────────────

#[test]
fn reads_atoms() {
    assert_eq!(read_one("nil").unwrap(), Value::Nil);
    assert_eq!(read_one("true").unwrap(), Value::Bool(true));
    assert_eq!(read_one("false").unwrap(), Value::Bool(false));
    assert_eq!(read_one("42").unwrap(), Value::Int(42));
    assert_eq!(read_one("-3").unwrap(), Value::Int(-3));
    assert_eq!(read_one("1.5").unwrap(), Value::Float(1.5));
    assert_eq!(read_one("1e3").unwrap(), Value::Float(1000.0));
    assert_eq!(read_one(":status").unwrap(), Value::kw("status"));
    assert_eq!(read_one("title").unwrap(), Value::sym("title"));
    assert_eq!(read_one("\"hi\\n\"").unwrap(), Value::str("hi\n"));
}

#[test]
fn nan_and_inf_are_symbols_not_numbers() {
    // The float parser must not swallow these identifiers.
    assert_eq!(read_one("nan").unwrap(), Value::sym("nan"));
    assert_eq!(read_one("inf").unwrap(), Value::sym("inf"));
    assert_eq!(read_one("->").unwrap(), Value::sym("->"));
    assert_eq!(read_one("-").unwrap(), Value::sym("-"));
}

#[test]
fn reads_collections() {
    assert_eq!(
        read_one("(+ 1 2)").unwrap(),
        Value::list(vec![Value::sym("+"), Value::Int(1), Value::Int(2)])
    );
    assert_eq!(
        read_one("[1 2 3]").unwrap(),
        Value::vector(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
    // Commas are whitespace.
    assert_eq!(read_one("[1, 2, 3]").unwrap(), read_one("[1 2 3]").unwrap());
}

#[test]
fn reader_sugar_expands() {
    assert_eq!(
        read_one("'x").unwrap(),
        Value::list(vec![Value::sym("quote"), Value::sym("x")])
    );
    assert_eq!(
        read_one("`(a ~b ~@c)").unwrap(),
        read_one("(quasiquote (a (unquote b) (unquote-splicing c)))").unwrap()
    );
    // #(…) shorthand.
    assert_eq!(
        read_one("#(+ % 1)").unwrap(),
        read_one("(fn [%1] (+ %1 1))").unwrap()
    );
    assert_eq!(
        read_one("#(+ %1 %2)").unwrap(),
        read_one("(fn [%1 %2] (+ %1 %2))").unwrap()
    );
}

#[test]
fn reads_tagged_literals() {
    let v = read_one("#uuid \"abc\"").unwrap();
    match v {
        Value::Tagged { tag, value } => {
            assert_eq!(&*tag, "uuid");
            assert_eq!(*value, Value::str("abc"));
        }
        other => panic!("expected tagged, got {other}"),
    }
}

#[test]
fn line_comments_ignored() {
    assert_eq!(eval("; a comment\n(+ 1 2) ; trailing"), Value::Int(3));
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
    assert_eq!(eval("(+ 1 2 3)"), Value::Int(6));
    assert_eq!(eval("(- 10 3 2)"), Value::Int(5));
    assert_eq!(eval("(- 5)"), Value::Int(-5));
    assert_eq!(eval("(* 2 3 4)"), Value::Int(24));
    assert_eq!(eval("(/ 6 2)"), Value::Float(3.0)); // `/` always float
    assert_eq!(eval("(mod 7 3)"), Value::Int(1));
    assert_eq!(eval("(+ 1 2.0)"), Value::Float(3.0)); // int+float promotes
    assert_eq!(eval("(max 3 7 2)"), Value::Int(7));
    assert_eq!(eval("(floor 3.7)"), Value::Int(3));
}

#[test]
fn equality_is_numeric_cross_type() {
    assert_eq!(eval("(= 1 1.0)"), Value::Bool(true));
    assert_eq!(eval("(= 1 2)"), Value::Bool(false));
    assert_eq!(eval("(= \"a\" \"a\")"), Value::Bool(true));
    assert_eq!(eval("(= [1 2] [1 2])"), Value::Bool(true));
    assert_eq!(eval("(not= 1 2)"), Value::Bool(true));
}

#[test]
fn comparison_chains() {
    assert_eq!(eval("(< 1 2 3)"), Value::Bool(true));
    assert_eq!(eval("(< 1 3 2)"), Value::Bool(false));
    assert_eq!(eval("(>= 3 3 1)"), Value::Bool(true));
}

#[test]
fn divide_by_zero_is_an_error() {
    assert!(matches!(try_eval("(/ 1 0)"), Err(LdError::DivideByZero)));
    assert!(matches!(try_eval("(mod 1 0)"), Err(LdError::DivideByZero)));
}

// ── Truthiness & special forms ──────────────────────────────────────────────

#[test]
fn truthiness() {
    assert_eq!(eval("(if 0 :yes :no)"), Value::kw("yes")); // 0 is truthy
    assert_eq!(eval("(if \"\" :yes :no)"), Value::kw("yes")); // "" is truthy
    assert_eq!(eval("(if nil :yes :no)"), Value::kw("no"));
    assert_eq!(eval("(if false :yes :no)"), Value::kw("no"));
    assert_eq!(eval("(if true :yes)"), Value::kw("yes"));
    assert_eq!(eval("(if false :yes)"), Value::Nil); // no else → nil
}

#[test]
fn let_bindings_are_sequential() {
    assert_eq!(eval("(let [x 1 y (+ x 1)] (* x y))"), Value::Int(2));
}

#[test]
fn do_returns_last() {
    assert_eq!(eval("(do 1 2 3)"), Value::Int(3));
    assert_eq!(eval("(do)"), Value::Nil);
}

#[test]
fn def_binds_globally() {
    let mut i = Interp::new();
    i.eval_str("(def x 41)", Budget::new()).unwrap();
    assert_eq!(
        i.eval_str("(+ x 1)", Budget::new()).unwrap(),
        Value::Int(42)
    );
}

#[test]
fn closures_and_recursion() {
    let prog = "(def fact (fn f [n] (if (<= n 1) 1 (* n (f (- n 1)))))) (fact 5)";
    assert_eq!(eval(prog), Value::Int(120));
}

#[test]
fn closures_capture_environment() {
    let prog = "(def make-adder (fn [n] (fn [x] (+ x n)))) (def add10 (make-adder 10)) (add10 5)";
    assert_eq!(eval(prog), Value::Int(15));
}

#[test]
fn variadic_rest_param() {
    assert_eq!(eval("((fn [a & rest] rest) 1 2 3)"), eval("[2 3]"));
    assert_eq!(eval("((fn [a & rest] a) 1 2 3)"), Value::Int(1));
}

// ── Higher-order & collections ──────────────────────────────────────────────

#[test]
fn higher_order_sequence_ops() {
    assert_eq!(eval("(map #(* % %) [1 2 3])"), eval("[1 4 9]"));
    assert_eq!(eval("(filter #(> % 2) [1 2 3 4])"), eval("[3 4]"));
    assert_eq!(eval("(reduce + 0 [1 2 3 4])"), Value::Int(10));
    assert_eq!(eval("(reduce + [1 2 3 4])"), Value::Int(10));
    assert_eq!(eval("(count [1 2 3])"), Value::Int(3));
    assert_eq!(eval("(first [1 2 3])"), Value::Int(1));
    assert_eq!(eval("(rest [1 2 3])"), eval("[2 3]"));
    assert_eq!(eval("(reverse [1 2 3])"), eval("[3 2 1]"));
    assert_eq!(eval("(sort [3 1 2])"), eval("[1 2 3]"));
    assert_eq!(eval("(range 4)"), eval("[0 1 2 3]"));
}

#[test]
fn threading_via_nested_calls() {
    // (take 2 (sort-by identity ...)) — sort-by with a keyword key.
    assert_eq!(
        eval("(sort-by :n [{:n 3} {:n 1} {:n 2}])"),
        eval("[{:n 1} {:n 2} {:n 3}]")
    );
}

#[test]
fn keyword_as_function() {
    assert_eq!(eval("(:status {:status \"todo\"})"), Value::str("todo"));
    assert_eq!(eval("(:missing {:status \"todo\"})"), Value::Nil);
    assert_eq!(eval("(:missing {:a 1} :default)"), Value::kw("default"));
    assert_eq!(eval("(:a {:a 1} \"fallback\")"), Value::Int(1)); // present → not the default
}

#[test]
fn maps_and_sets() {
    assert_eq!(eval("(get {:a 1 :b 2} :b)"), Value::Int(2));
    assert_eq!(eval("(assoc {:a 1} :b 2)"), eval("{:a 1 :b 2}"));
    assert_eq!(eval("(dissoc {:a 1 :b 2} :a)"), eval("{:b 2}"));
    assert_eq!(eval("(keys {:a 1 :b 2})"), eval("[:a :b]"));
    assert_eq!(eval("(contains? #{:a :b} :a)"), Value::Bool(true));
    assert_eq!(eval("(union #{1 2} #{2 3})"), eval("#{1 2 3}"));
    assert_eq!(eval("(intersection #{1 2 3} #{2 3 4})"), eval("#{2 3}"));
    assert_eq!(eval("(count #{1 1 2})"), Value::Int(2)); // set dedups
}

#[test]
fn string_ops() {
    assert_eq!(eval("(str \"a\" 1 :b)"), Value::str("a1:b"));
    assert_eq!(eval("(upper \"hi\")"), Value::str("HI"));
    assert_eq!(eval("(join \", \" [\"a\" \"b\"])"), Value::str("a, b"));
    assert_eq!(eval("(count (words \"a b c\"))"), Value::Int(3));
    assert_eq!(eval("(includes? \"hello\" \"ell\")"), Value::Bool(true));
    assert_eq!(eval("(subs \"hello\" 1 3)"), Value::str("el"));
}

#[test]
fn collection_literals_evaluate_elements() {
    assert_eq!(eval("[1 (+ 1 1) 3]"), eval("[1 2 3]"));
    assert_eq!(eval("{:sum (+ 1 2)}"), eval("{:sum 3}"));
    // But quote keeps them literal.
    assert_eq!(
        eval("'(+ 1 2)"),
        Value::list(vec![Value::sym("+"), Value::Int(1), Value::Int(2)])
    );
}

// ── Quasiquote ──────────────────────────────────────────────────────────────

#[test]
fn quasiquote_unquote_and_splice() {
    let prog = "(let [x 2 xs [3 4]] `(1 ~x ~@xs 5))";
    assert_eq!(eval(prog), eval("[1 2 3 4 5]").pipe_to_list());
}

// ── Reflection ──────────────────────────────────────────────────────────────

#[test]
fn eval_and_read_reflection() {
    assert_eq!(eval("(eval '(+ 1 2))"), Value::Int(3));
    assert_eq!(eval("(read \"(+ 1 2)\")"), eval("'(+ 1 2)"));
    assert_eq!(eval("(type 5)"), Value::kw("int"));
    assert_eq!(eval("(type :x)"), Value::kw("keyword"));
}

// ── Errors as values ────────────────────────────────────────────────────────

#[test]
fn errors_are_structured_not_panics() {
    assert!(matches!(
        try_eval("undefined-symbol"),
        Err(LdError::Unbound(_))
    ));
    assert!(matches!(try_eval("(1 2 3)"), Err(LdError::NotCallable(_))));
    assert!(matches!(try_eval("(+ 1 \"a\")"), Err(LdError::Type { .. })));
    assert!(matches!(try_eval("(if)"), Err(LdError::Syntax { .. })));
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
    assert_eq!(eval(prog), eval(prog));
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
        let back = read_one(&printed)
            .unwrap_or_else(|e| panic!("could not re-read {printed:?}: {e}"));
        prop_assert_eq!(back, v);
    }
}

// A tiny helper to keep one quasiquote assertion honest: `(1 2 …)` builds a
// list, while `[1 2 …]` builds a vector; value-equality treats them as equal,
// but we make the intent explicit here.
trait PipeToList {
    fn pipe_to_list(self) -> Value;
}
impl PipeToList for Value {
    fn pipe_to_list(self) -> Value {
        match self {
            Value::Vector(xs) => Value::List(xs),
            other => other,
        }
    }
}
