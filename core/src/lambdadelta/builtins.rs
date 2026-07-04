// SPDX-License-Identifier: MPL-2.0
//! The kernel's *pure* builtin vocabulary (spec §4).
//!
//! These are the note-agnostic primitives every host inherits for free:
//! arithmetic, comparison, predicates, sequence/set/map/string helpers, and a
//! little reflection (`eval`, `read`). Notebook readers and `!`-mutators are
//! **not** here — a host registers those through [`Interp::register_builtin`]
//! (spec §7). Nothing in this file knows what a note is.
//!
//! Collection *transformers* (`map`, `filter`, `rest`, `sort`, …) return
//! **vectors** — the spec's "data" form (§9.4) — which is friendlier for a note
//! tool than Clojure's lazy seqs. `conj`/`cons` preserve the natural structure.

use std::rc::Rc;

use super::error::{LdError, LdResult};
use super::value::{map_lookup, set_contains, value_eq, Value};
use super::Interp;

/// Install every kernel builtin into `interp`'s global environment.
pub fn install(interp: &mut Interp) {
    // Arithmetic.
    interp.register_builtin("+", 0, None, add);
    interp.register_builtin("-", 1, None, sub);
    interp.register_builtin("*", 0, None, mul);
    interp.register_builtin("/", 1, None, div);
    interp.register_builtin("mod", 2, Some(2), rem);
    interp.register_builtin("min", 1, None, min_fn);
    interp.register_builtin("max", 1, None, max_fn);
    interp.register_builtin("abs", 1, Some(1), abs_fn);
    interp.register_builtin("floor", 1, Some(1), |_, a| round_like(&a[0], f64::floor));
    interp.register_builtin("ceil", 1, Some(1), |_, a| round_like(&a[0], f64::ceil));
    interp.register_builtin("round", 1, Some(1), |_, a| round_like(&a[0], f64::round));

    // Comparison & logic.
    interp.register_builtin("=", 1, None, |_, a| Ok(Value::Bool(all_equal(a))));
    interp.register_builtin("not=", 1, None, |_, a| Ok(Value::Bool(!all_equal(a))));
    interp.register_builtin("<", 1, None, |_, a| chain_cmp(a, |o| o.is_lt()));
    interp.register_builtin(">", 1, None, |_, a| chain_cmp(a, |o| o.is_gt()));
    interp.register_builtin("<=", 1, None, |_, a| chain_cmp(a, |o| o.is_le()));
    interp.register_builtin(">=", 1, None, |_, a| chain_cmp(a, |o| o.is_ge()));
    interp.register_builtin("not", 1, Some(1), |_, a| Ok(Value::Bool(!a[0].is_truthy())));

    // Predicates.
    interp.register_builtin("nil?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Nil))
    });
    interp.register_builtin("true?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Bool(true)))
    });
    interp.register_builtin("false?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Bool(false)))
    });
    interp.register_builtin("number?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Int(_) | Value::Float(_)))
    });
    interp.register_builtin("string?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Str(_)))
    });
    interp.register_builtin("symbol?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Symbol(_)))
    });
    interp.register_builtin("keyword?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Keyword(_)))
    });
    interp.register_builtin("list?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::List(_)))
    });
    interp.register_builtin("vector?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Vector(_)))
    });
    interp.register_builtin("set?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Set(_)))
    });
    interp.register_builtin("map?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Map(_)))
    });
    interp.register_builtin("fn?", 1, Some(1), |_, a| {
        pred(a, |v| matches!(v, Value::Fn(_) | Value::Builtin(_)))
    });
    interp.register_builtin("empty?", 1, Some(1), |_, a| {
        Ok(Value::Bool(is_empty(&a[0])))
    });

    // Sequences.
    interp.register_builtin("list", 0, None, |_, a| Ok(Value::list(a.to_vec())));
    interp.register_builtin("vector", 0, None, |_, a| Ok(Value::vector(a.to_vec())));
    interp.register_builtin("count", 1, Some(1), count_fn);
    interp.register_builtin("first", 1, Some(1), first_fn);
    interp.register_builtin("last", 1, Some(1), last_fn);
    interp.register_builtin("rest", 1, Some(1), rest_fn);
    interp.register_builtin("nth", 2, Some(3), nth_fn);
    interp.register_builtin("get", 2, Some(3), get_fn);
    interp.register_builtin("take", 2, Some(2), take_fn);
    interp.register_builtin("drop", 2, Some(2), drop_fn);
    interp.register_builtin("reverse", 1, Some(1), |_, a| {
        let mut v = seq_items("reverse", &a[0])?;
        v.reverse();
        Ok(Value::vector(v))
    });
    interp.register_builtin("range", 1, Some(3), range_fn);
    interp.register_builtin("conj", 1, None, conj_fn);
    interp.register_builtin("cons", 2, Some(2), cons_fn);
    interp.register_builtin("concat", 0, None, concat_fn);
    interp.register_builtin("distinct", 1, Some(1), distinct_fn);
    interp.register_builtin("into", 2, Some(2), into_fn);
    interp.register_builtin("sort", 1, Some(1), sort_fn);
    interp.register_builtin("map", 2, Some(2), map_fn);
    interp.register_builtin("filter", 2, Some(2), |i, a| keep_fn(i, a, true));
    interp.register_builtin("remove", 2, Some(2), |i, a| keep_fn(i, a, false));
    interp.register_builtin("reduce", 2, Some(3), reduce_fn);
    interp.register_builtin("sort-by", 2, Some(2), sort_by_fn);
    interp.register_builtin("some", 2, Some(2), some_fn);
    interp.register_builtin("every?", 2, Some(2), every_fn);

    // Sets.
    interp.register_builtin("set", 1, Some(1), |_, a| {
        Ok(make_set(seq_items("set", &a[0])?))
    });
    interp.register_builtin("union", 0, None, union_fn);
    interp.register_builtin("intersection", 1, None, intersection_fn);
    interp.register_builtin("difference", 1, None, difference_fn);
    interp.register_builtin("subset?", 2, Some(2), subset_fn);
    interp.register_builtin("disj", 1, None, disj_fn);
    interp.register_builtin("contains?", 2, Some(2), contains_fn);

    // Strings.
    interp.register_builtin("str", 0, None, |_, a| {
        Ok(Value::str(a.iter().map(to_str_raw).collect::<String>()))
    });
    interp.register_builtin("join", 2, Some(2), join_fn);
    interp.register_builtin("split", 2, Some(2), split_fn);
    interp.register_builtin("lines", 1, Some(1), |_, a| {
        let s = want_str("lines", &a[0])?;
        Ok(Value::vector(s.lines().map(Value::str).collect()))
    });
    interp.register_builtin("words", 1, Some(1), |_, a| {
        let s = want_str("words", &a[0])?;
        Ok(Value::vector(
            s.split_whitespace().map(Value::str).collect(),
        ))
    });
    interp.register_builtin("trim", 1, Some(1), |_, a| {
        Ok(Value::str(want_str("trim", &a[0])?.trim()))
    });
    interp.register_builtin("lower", 1, Some(1), |_, a| {
        Ok(Value::str(want_str("lower", &a[0])?.to_lowercase()))
    });
    interp.register_builtin("upper", 1, Some(1), |_, a| {
        Ok(Value::str(want_str("upper", &a[0])?.to_uppercase()))
    });
    interp.register_builtin("starts-with?", 2, Some(2), |_, a| {
        Ok(Value::Bool(
            want_str("starts-with?", &a[0])?.starts_with(want_str("starts-with?", &a[1])?),
        ))
    });
    interp.register_builtin("ends-with?", 2, Some(2), |_, a| {
        Ok(Value::Bool(
            want_str("ends-with?", &a[0])?.ends_with(want_str("ends-with?", &a[1])?),
        ))
    });
    interp.register_builtin("includes?", 2, Some(2), |_, a| {
        Ok(Value::Bool(
            want_str("includes?", &a[0])?.contains(want_str("includes?", &a[1])?),
        ))
    });
    interp.register_builtin("replace", 3, Some(3), |_, a| {
        let s = want_str("replace", &a[0])?;
        Ok(Value::str(s.replace(
            want_str("replace", &a[1])?,
            want_str("replace", &a[2])?,
        )))
    });
    interp.register_builtin("subs", 2, Some(3), subs_fn);

    // Maps.
    interp.register_builtin("assoc", 3, None, assoc_fn);
    interp.register_builtin("dissoc", 1, None, dissoc_fn);
    interp.register_builtin("keys", 1, Some(1), |_, a| {
        Ok(Value::vector(
            want_map("keys", &a[0])?
                .iter()
                .map(|(k, _)| k.clone())
                .collect(),
        ))
    });
    interp.register_builtin("vals", 1, Some(1), |_, a| {
        Ok(Value::vector(
            want_map("vals", &a[0])?
                .iter()
                .map(|(_, v)| v.clone())
                .collect(),
        ))
    });
    interp.register_builtin("merge", 0, None, merge_fn);
    interp.register_builtin("select-keys", 2, Some(2), select_keys_fn);
    interp.register_builtin("update", 3, None, update_fn);

    // Reflection (homoiconicity).
    interp.register_builtin("eval", 1, Some(1), |i, a| {
        let g = i.global.clone();
        i.eval(&a[0], &g)
    });
    interp.register_builtin("read", 1, Some(1), |_, a| {
        super::reader::read_one(want_str("read", &a[0])?)
    });
    interp.register_builtin("type", 1, Some(1), |_, a| Ok(Value::kw(a[0].type_name())));
}

// ── Numeric helpers ──────────────────────────────────────────────────────────

enum Num {
    Int(i64),
    Float(f64),
}

fn as_num(op: &str, v: &Value) -> LdResult<Num> {
    match v {
        Value::Int(n) => Ok(Num::Int(*n)),
        Value::Float(x) => Ok(Num::Float(*x)),
        other => Err(LdError::Type {
            op: op.to_string(),
            expected: "number".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

fn num_f64(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::Float(f) => *f,
    }
}

fn add(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut int_acc: i64 = 0;
    let mut float_acc: f64 = 0.0;
    let mut is_float = false;
    for a in args {
        match as_num("+", a)? {
            Num::Int(n) if !is_float => match int_acc.checked_add(n) {
                Some(v) => int_acc = v,
                None => {
                    is_float = true;
                    float_acc = int_acc as f64 + n as f64;
                }
            },
            Num::Int(n) => float_acc += n as f64,
            Num::Float(x) => {
                if !is_float {
                    is_float = true;
                    float_acc = int_acc as f64;
                }
                float_acc += x;
            }
        }
    }
    Ok(if is_float {
        Value::Float(float_acc)
    } else {
        Value::Int(int_acc)
    })
}

fn mul(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut int_acc: i64 = 1;
    let mut float_acc: f64 = 1.0;
    let mut is_float = false;
    for a in args {
        match as_num("*", a)? {
            Num::Int(n) if !is_float => match int_acc.checked_mul(n) {
                Some(v) => int_acc = v,
                None => {
                    is_float = true;
                    float_acc = int_acc as f64 * n as f64;
                }
            },
            Num::Int(n) => float_acc *= n as f64,
            Num::Float(x) => {
                if !is_float {
                    is_float = true;
                    float_acc = int_acc as f64;
                }
                float_acc *= x;
            }
        }
    }
    Ok(if is_float {
        Value::Float(float_acc)
    } else {
        Value::Int(int_acc)
    })
}

fn sub(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let first = as_num("-", &args[0])?;
    if args.len() == 1 {
        return Ok(match first {
            Num::Int(n) => Value::Int(-n),
            Num::Float(x) => Value::Float(-x),
        });
    }
    let mut is_float = matches!(first, Num::Float(_));
    let mut int_acc = if let Num::Int(n) = first { n } else { 0 };
    let mut float_acc = num_f64(&first);
    for a in &args[1..] {
        match as_num("-", a)? {
            Num::Int(n) if !is_float => match int_acc.checked_sub(n) {
                Some(v) => int_acc = v,
                None => {
                    is_float = true;
                    float_acc = int_acc as f64 - n as f64;
                }
            },
            Num::Int(n) => float_acc -= n as f64,
            Num::Float(x) => {
                if !is_float {
                    is_float = true;
                    float_acc = int_acc as f64;
                }
                float_acc -= x;
            }
        }
    }
    Ok(if is_float {
        Value::Float(float_acc)
    } else {
        Value::Int(int_acc)
    })
}

fn div(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    // Spec §2: `/` produces floats.
    let first = num_f64(&as_num("/", &args[0])?);
    if args.len() == 1 {
        if first == 0.0 {
            return Err(LdError::DivideByZero);
        }
        return Ok(Value::Float(1.0 / first));
    }
    let mut acc = first;
    for a in &args[1..] {
        let d = num_f64(&as_num("/", a)?);
        if d == 0.0 {
            return Err(LdError::DivideByZero);
        }
        acc /= d;
    }
    Ok(Value::Float(acc))
}

fn rem(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => {
            if *b == 0 {
                Err(LdError::DivideByZero)
            } else {
                Ok(Value::Int(a.rem_euclid(*b)))
            }
        }
        _ => {
            let a = num_f64(&as_num("mod", &args[0])?);
            let b = num_f64(&as_num("mod", &args[1])?);
            if b == 0.0 {
                Err(LdError::DivideByZero)
            } else {
                Ok(Value::Float(a.rem_euclid(b)))
            }
        }
    }
}

fn min_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut best = args[0].clone();
    for a in &args[1..] {
        if num_cmp("min", a, &best)?.is_lt() {
            best = a.clone();
        }
    }
    as_num("min", &best).map(|_| best)
}

fn max_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut best = args[0].clone();
    for a in &args[1..] {
        if num_cmp("max", a, &best)?.is_gt() {
            best = a.clone();
        }
    }
    as_num("max", &best).map(|_| best)
}

fn abs_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    Ok(match as_num("abs", &args[0])? {
        Num::Int(n) => Value::Int(n.abs()),
        Num::Float(x) => Value::Float(x.abs()),
    })
}

fn round_like(v: &Value, f: fn(f64) -> f64) -> LdResult<Value> {
    match v {
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Float(x) => Ok(Value::Int(f(*x) as i64)),
        other => Err(LdError::Type {
            op: "round".to_string(),
            expected: "number".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

/// Numeric comparison keeping integer precision when both sides are integers.
fn num_cmp(op: &str, a: &Value, b: &Value) -> LdResult<std::cmp::Ordering> {
    match (as_num(op, a)?, as_num(op, b)?) {
        (Num::Int(x), Num::Int(y)) => Ok(x.cmp(&y)),
        (x, y) => Ok(num_f64(&x).total_cmp(&num_f64(&y))),
    }
}

fn chain_cmp(args: &[Value], ok: fn(std::cmp::Ordering) -> bool) -> LdResult<Value> {
    for pair in args.windows(2) {
        if !ok(num_cmp("compare", &pair[0], &pair[1])?) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn all_equal(args: &[Value]) -> bool {
    args.windows(2).all(|p| value_eq(&p[0], &p[1]))
}

fn pred(args: &[Value], f: fn(&Value) -> bool) -> LdResult<Value> {
    Ok(Value::Bool(f(&args[0])))
}

// ── Sequence helpers ─────────────────────────────────────────────────────────

/// Coerce a value into a flat item list for sequence operations. Maps yield
/// `[k v]` pairs; `nil` is the empty sequence.
fn seq_items(op: &str, v: &Value) -> LdResult<Vec<Value>> {
    match v {
        Value::List(xs) | Value::Vector(xs) | Value::Set(xs) => Ok(xs.as_ref().clone()),
        Value::Map(pairs) => Ok(pairs
            .iter()
            .map(|(k, val)| Value::vector(vec![k.clone(), val.clone()]))
            .collect()),
        Value::Nil => Ok(Vec::new()),
        other => Err(LdError::Type {
            op: op.to_string(),
            expected: "sequence".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Nil => true,
        Value::Str(s) => s.is_empty(),
        Value::List(xs) | Value::Vector(xs) | Value::Set(xs) => xs.is_empty(),
        Value::Map(pairs) => pairs.is_empty(),
        _ => false,
    }
}

fn count_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let n = match &args[0] {
        Value::Str(s) => s.chars().count(),
        Value::Nil => 0,
        other => seq_items("count", other)?.len(),
    };
    Ok(Value::Int(n as i64))
}

fn first_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    Ok(seq_items("first", &args[0])?
        .into_iter()
        .next()
        .unwrap_or(Value::Nil))
}

fn last_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    Ok(seq_items("last", &args[0])?.pop().unwrap_or(Value::Nil))
}

fn rest_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let items = seq_items("rest", &args[0])?;
    Ok(Value::vector(items.into_iter().skip(1).collect()))
}

fn nth_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let items = seq_items("nth", &args[0])?;
    let idx = want_int("nth", &args[1])?;
    if idx >= 0 && (idx as usize) < items.len() {
        Ok(items[idx as usize].clone())
    } else if let Some(default) = args.get(2) {
        Ok(default.clone())
    } else {
        Err(LdError::Type {
            op: "nth".to_string(),
            expected: format!("index in 0..{}", items.len()),
            got: idx.to_string(),
        })
    }
}

fn get_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let default = args.get(2).cloned().unwrap_or(Value::Nil);
    match &args[0] {
        Value::Map(pairs) => Ok(map_lookup(pairs, &args[1]).cloned().unwrap_or(default)),
        Value::Vector(xs) | Value::List(xs) => match &args[1] {
            Value::Int(n) if *n >= 0 && (*n as usize) < xs.len() => Ok(xs[*n as usize].clone()),
            _ => Ok(default),
        },
        Value::Set(xs) => {
            if set_contains(xs, &args[1]) {
                Ok(args[1].clone())
            } else {
                Ok(default)
            }
        }
        Value::Nil => Ok(default),
        _ => Ok(default),
    }
}

fn take_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let n = want_int("take", &args[0])?.max(0) as usize;
    Ok(Value::vector(
        seq_items("take", &args[1])?.into_iter().take(n).collect(),
    ))
}

fn drop_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let n = want_int("drop", &args[0])?.max(0) as usize;
    Ok(Value::vector(
        seq_items("drop", &args[1])?.into_iter().skip(n).collect(),
    ))
}

fn range_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let (start, end, step) = match args.len() {
        1 => (0, want_int("range", &args[0])?, 1),
        2 => (
            want_int("range", &args[0])?,
            want_int("range", &args[1])?,
            1,
        ),
        _ => (
            want_int("range", &args[0])?,
            want_int("range", &args[1])?,
            want_int("range", &args[2])?,
        ),
    };
    if step == 0 {
        return Err(LdError::Syntax {
            form: "range".to_string(),
            msg: "step must be non-zero".to_string(),
        });
    }
    let mut out = Vec::new();
    let mut i = start;
    // Bounded independently of the budget's per-step charge.
    while (step > 0 && i < end) || (step < 0 && i > end) {
        out.push(Value::Int(i));
        if out.len() > 1_000_000 {
            return Err(LdError::Budget(
                "range produced too many elements".to_string(),
            ));
        }
        i += step;
    }
    Ok(Value::vector(out))
}

fn conj_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut coll = args[0].clone();
    for x in &args[1..] {
        coll = match coll {
            Value::Vector(xs) => {
                let mut v = xs.as_ref().clone();
                v.push(x.clone());
                Value::vector(v)
            }
            Value::List(xs) => {
                let mut v = vec![x.clone()];
                v.extend(xs.as_ref().clone());
                Value::list(v)
            }
            Value::Set(xs) => {
                let mut v = xs.as_ref().clone();
                if !set_contains(&v, x) {
                    v.push(x.clone());
                }
                Value::Set(Rc::new(v))
            }
            Value::Map(pairs) => {
                let entry = seq_items("conj", x)?;
                if entry.len() != 2 {
                    return Err(LdError::Type {
                        op: "conj".to_string(),
                        expected: "a [k v] pair".to_string(),
                        got: x.type_name().to_string(),
                    });
                }
                Value::Map(Rc::new(assoc_pairs(
                    &pairs,
                    entry[0].clone(),
                    entry[1].clone(),
                )))
            }
            Value::Nil => Value::vector(vec![x.clone()]),
            other => {
                return Err(LdError::Type {
                    op: "conj".to_string(),
                    expected: "collection".to_string(),
                    got: other.type_name().to_string(),
                })
            }
        };
    }
    Ok(coll)
}

fn cons_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut v = vec![args[0].clone()];
    v.extend(seq_items("cons", &args[1])?);
    Ok(Value::list(v))
}

fn concat_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut out = Vec::new();
    for a in args {
        out.extend(seq_items("concat", a)?);
    }
    Ok(Value::vector(out))
}

fn distinct_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut out: Vec<Value> = Vec::new();
    for x in seq_items("distinct", &args[0])? {
        if !out.iter().any(|y| value_eq(y, &x)) {
            out.push(x);
        }
    }
    Ok(Value::vector(out))
}

fn into_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let from = seq_items("into", &args[1])?;
    match &args[0] {
        Value::Vector(xs) => {
            let mut v = xs.as_ref().clone();
            v.extend(from);
            Ok(Value::vector(v))
        }
        Value::List(xs) => {
            let mut v = xs.as_ref().clone();
            for x in from {
                v.insert(0, x);
            }
            Ok(Value::list(v))
        }
        Value::Set(_) => Ok(make_set(from)),
        Value::Map(pairs) => {
            let mut acc = pairs.as_ref().clone();
            for pair in from {
                let kv = seq_items("into", &pair)?;
                if kv.len() != 2 {
                    return Err(LdError::Type {
                        op: "into".to_string(),
                        expected: "[k v] pairs".to_string(),
                        got: pair.type_name().to_string(),
                    });
                }
                acc = assoc_pairs(&acc, kv[0].clone(), kv[1].clone());
            }
            Ok(Value::Map(Rc::new(acc)))
        }
        Value::Nil => Ok(Value::vector(from)),
        other => Err(LdError::Type {
            op: "into".to_string(),
            expected: "collection".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

fn sort_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut items = seq_items("sort", &args[0])?;
    sort_values(&mut items)?;
    Ok(Value::vector(items))
}

fn map_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let f = args[0].clone();
    let items = seq_items("map", &args[1])?;
    let mut out = Vec::with_capacity(items.len());
    for x in items {
        out.push(i.apply(&f, &[x])?);
    }
    Ok(Value::vector(out))
}

fn keep_fn(i: &mut Interp, args: &[Value], keep_truthy: bool) -> LdResult<Value> {
    let f = args[0].clone();
    let items = seq_items("filter", &args[1])?;
    let mut out = Vec::new();
    for x in items {
        if i.apply(&f, std::slice::from_ref(&x))?.is_truthy() == keep_truthy {
            out.push(x);
        }
    }
    Ok(Value::vector(out))
}

fn reduce_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let f = args[0].clone();
    if args.len() == 2 {
        let items = seq_items("reduce", &args[1])?;
        let mut it = items.into_iter();
        let Some(mut acc) = it.next() else {
            return i.apply(&f, &[]);
        };
        for x in it {
            acc = i.apply(&f, &[acc, x])?;
        }
        Ok(acc)
    } else {
        let mut acc = args[1].clone();
        for x in seq_items("reduce", &args[2])? {
            acc = i.apply(&f, &[acc, x])?;
        }
        Ok(acc)
    }
}

fn sort_by_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let f = args[0].clone();
    let items = seq_items("sort-by", &args[1])?;
    let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(items.len());
    for x in items {
        let k = i.apply(&f, std::slice::from_ref(&x))?;
        keyed.push((k, x));
    }
    let mut keys: Vec<Value> = keyed.iter().map(|(k, _)| k.clone()).collect();
    let kind = sort_kind(&keys)?;
    keyed.sort_by(|a, b| cmp_same_kind(&a.0, &b.0, kind));
    let _ = &mut keys;
    Ok(Value::vector(keyed.into_iter().map(|(_, x)| x).collect()))
}

fn some_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let f = args[0].clone();
    for x in seq_items("some", &args[1])? {
        let r = i.apply(&f, &[x])?;
        if r.is_truthy() {
            return Ok(r);
        }
    }
    Ok(Value::Nil)
}

fn every_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let f = args[0].clone();
    for x in seq_items("every?", &args[1])? {
        if !i.apply(&f, &[x])?.is_truthy() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

// ── Sets ─────────────────────────────────────────────────────────────────────

fn make_set(items: Vec<Value>) -> Value {
    let mut members: Vec<Value> = Vec::with_capacity(items.len());
    for x in items {
        if !set_contains(&members, &x) {
            members.push(x);
        }
    }
    Value::Set(Rc::new(members))
}

fn union_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut acc: Vec<Value> = Vec::new();
    for a in args {
        for x in seq_items("union", a)? {
            if !set_contains(&acc, &x) {
                acc.push(x);
            }
        }
    }
    Ok(Value::Set(Rc::new(acc)))
}

fn intersection_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let first = seq_items("intersection", &args[0])?;
    let mut acc: Vec<Value> = Vec::new();
    for x in first {
        let in_all = args[1..]
            .iter()
            .map(|a| seq_items("intersection", a))
            .collect::<LdResult<Vec<_>>>()?
            .iter()
            .all(|other| set_contains(other, &x));
        if in_all && !set_contains(&acc, &x) {
            acc.push(x);
        }
    }
    Ok(Value::Set(Rc::new(acc)))
}

fn difference_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut acc = make_members(seq_items("difference", &args[0])?);
    for a in &args[1..] {
        let remove = seq_items("difference", a)?;
        acc.retain(|x| !set_contains(&remove, x));
    }
    Ok(Value::Set(Rc::new(acc)))
}

fn subset_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let sub = seq_items("subset?", &args[0])?;
    let sup = seq_items("subset?", &args[1])?;
    Ok(Value::Bool(sub.iter().all(|x| set_contains(&sup, x))))
}

fn disj_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut members = match &args[0] {
        Value::Set(xs) => xs.as_ref().clone(),
        other => {
            return Err(LdError::Type {
                op: "disj".to_string(),
                expected: "set".to_string(),
                got: other.type_name().to_string(),
            })
        }
    };
    for x in &args[1..] {
        members.retain(|m| !value_eq(m, x));
    }
    Ok(Value::Set(Rc::new(members)))
}

fn contains_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let found = match &args[0] {
        Value::Set(xs) => set_contains(xs, &args[1]),
        Value::Map(pairs) => map_lookup(pairs, &args[1]).is_some(),
        Value::Vector(xs) | Value::List(xs) => match &args[1] {
            Value::Int(n) => *n >= 0 && (*n as usize) < xs.len(),
            _ => false,
        },
        Value::Nil => false,
        other => {
            return Err(LdError::Type {
                op: "contains?".to_string(),
                expected: "set, map, or vector".to_string(),
                got: other.type_name().to_string(),
            })
        }
    };
    Ok(Value::Bool(found))
}

fn make_members(items: Vec<Value>) -> Vec<Value> {
    let mut acc: Vec<Value> = Vec::new();
    for x in items {
        if !set_contains(&acc, &x) {
            acc.push(x);
        }
    }
    acc
}

// ── Strings ──────────────────────────────────────────────────────────────────

/// A value's *content* string: raw for strings, empty for nil, printed form
/// otherwise. Used by `str`/`join`.
fn to_str_raw(v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        Value::Nil => String::new(),
        other => other.to_string(),
    }
}

fn join_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let sep = want_str("join", &args[0])?;
    let parts: Vec<String> = seq_items("join", &args[1])?
        .iter()
        .map(to_str_raw)
        .collect();
    Ok(Value::str(parts.join(sep)))
}

fn split_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let s = want_str("split", &args[0])?;
    let sep = want_str("split", &args[1])?;
    let parts: Vec<Value> = if sep.is_empty() {
        s.chars().map(|c| Value::str(c.to_string())).collect()
    } else {
        s.split(sep).map(Value::str).collect()
    };
    Ok(Value::vector(parts))
}

fn subs_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let s = want_str("subs", &args[0])?;
    let chars: Vec<char> = s.chars().collect();
    let start = want_int("subs", &args[1])?.max(0) as usize;
    let end = match args.get(2) {
        Some(v) => (want_int("subs", v)?.max(0) as usize).min(chars.len()),
        None => chars.len(),
    };
    if start > chars.len() || start > end {
        return Err(LdError::Type {
            op: "subs".to_string(),
            expected: format!("start ≤ end ≤ {}", chars.len()),
            got: format!("start {start}, end {end}"),
        });
    }
    Ok(Value::str(chars[start..end].iter().collect::<String>()))
}

// ── Maps ─────────────────────────────────────────────────────────────────────

fn assoc_pairs(pairs: &[(Value, Value)], key: Value, val: Value) -> Vec<(Value, Value)> {
    let mut out = pairs.to_vec();
    if let Some(slot) = out.iter_mut().find(|(k, _)| value_eq(k, &key)) {
        slot.1 = val;
    } else {
        out.push((key, val));
    }
    out
}

fn assoc_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    if !(args.len() - 1).is_multiple_of(2) {
        return Err(LdError::Arity {
            name: "assoc".to_string(),
            expected: "a map and an even number of k/v forms".to_string(),
            got: args.len(),
        });
    }
    let mut pairs = want_map("assoc", &args[0])?.to_vec();
    let mut i = 1;
    while i < args.len() {
        pairs = assoc_pairs(&pairs, args[i].clone(), args[i + 1].clone());
        i += 2;
    }
    Ok(Value::Map(Rc::new(pairs)))
}

fn dissoc_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut pairs = want_map("dissoc", &args[0])?.to_vec();
    for k in &args[1..] {
        pairs.retain(|(ek, _)| !value_eq(ek, k));
    }
    Ok(Value::Map(Rc::new(pairs)))
}

fn merge_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let mut acc: Vec<(Value, Value)> = Vec::new();
    for a in args {
        if matches!(a, Value::Nil) {
            continue;
        }
        for (k, v) in want_map("merge", a)? {
            acc = assoc_pairs(&acc, k.clone(), v.clone());
        }
    }
    Ok(Value::Map(Rc::new(acc)))
}

fn select_keys_fn(_i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    let pairs = want_map("select-keys", &args[0])?;
    let wanted = seq_items("select-keys", &args[1])?;
    let out: Vec<(Value, Value)> = pairs
        .iter()
        .filter(|(k, _)| wanted.iter().any(|w| value_eq(w, k)))
        .cloned()
        .collect();
    Ok(Value::Map(Rc::new(out)))
}

fn update_fn(i: &mut Interp, args: &[Value]) -> LdResult<Value> {
    // (update m k f & extra) → assoc m k (apply f (get m k) extra...)
    let pairs = want_map("update", &args[0])?.to_vec();
    let key = args[1].clone();
    let f = args[2].clone();
    let current = map_lookup(&pairs, &key).cloned().unwrap_or(Value::Nil);
    let mut call_args = vec![current];
    call_args.extend_from_slice(&args[3..]);
    let updated = i.apply(&f, &call_args)?;
    Ok(Value::Map(Rc::new(assoc_pairs(&pairs, key, updated))))
}

// ── Extraction helpers ───────────────────────────────────────────────────────

fn want_str<'a>(op: &str, v: &'a Value) -> LdResult<&'a str> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(LdError::Type {
            op: op.to_string(),
            expected: "string".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

fn want_int(op: &str, v: &Value) -> LdResult<i64> {
    match v {
        Value::Int(n) => Ok(*n),
        other => Err(LdError::Type {
            op: op.to_string(),
            expected: "integer".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

fn want_map<'a>(op: &str, v: &'a Value) -> LdResult<&'a [(Value, Value)]> {
    match v {
        Value::Map(pairs) => Ok(pairs),
        other => Err(LdError::Type {
            op: op.to_string(),
            expected: "map".to_string(),
            got: other.type_name().to_string(),
        }),
    }
}

// ── Sortable-value ordering (numbers or strings/keywords, homogeneous) ───────

#[derive(Clone, Copy, PartialEq)]
enum SortKind {
    Number,
    Text,
    Empty,
}

fn sort_kind(keys: &[Value]) -> LdResult<SortKind> {
    let mut kind = SortKind::Empty;
    for k in keys {
        let this = match k {
            Value::Int(_) | Value::Float(_) => SortKind::Number,
            Value::Str(_) | Value::Keyword(_) | Value::Symbol(_) => SortKind::Text,
            other => {
                return Err(LdError::Type {
                    op: "sort".to_string(),
                    expected: "numbers or strings".to_string(),
                    got: other.type_name().to_string(),
                })
            }
        };
        match kind {
            SortKind::Empty => kind = this,
            _ if kind == this => {}
            _ => {
                return Err(LdError::Type {
                    op: "sort".to_string(),
                    expected: "a single comparable kind".to_string(),
                    got: "mixed numbers and text".to_string(),
                })
            }
        }
    }
    Ok(kind)
}

fn cmp_same_kind(a: &Value, b: &Value, kind: SortKind) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match kind {
        SortKind::Number => {
            let fa = match a {
                Value::Int(n) => *n as f64,
                Value::Float(x) => *x,
                _ => 0.0,
            };
            let fb = match b {
                Value::Int(n) => *n as f64,
                Value::Float(x) => *x,
                _ => 0.0,
            };
            fa.total_cmp(&fb)
        }
        SortKind::Text => text_of(a).cmp(&text_of(b)),
        SortKind::Empty => Ordering::Equal,
    }
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        Value::Keyword(s) | Value::Symbol(s) => s.to_string(),
        other => other.to_string(),
    }
}

fn sort_values(items: &mut [Value]) -> LdResult<()> {
    let kind = sort_kind(items)?;
    items.sort_by(|a, b| cmp_same_kind(a, b, kind));
    Ok(())
}
