// SPDX-License-Identifier: MPL-2.0
//! The λδ value model and lexical environment (spec §2).
//!
//! A deliberately small, homoiconic set of values: code is data. Compound
//! values are reference-counted so cloning a `Value` is cheap — the evaluator
//! clones freely when moving values between scopes.
//!
//! The kernel is **note-agnostic** (spec §7): there is no `Note` variant here.
//! A "note" is simply a `Map` with an `:id`, given meaning by host builtins
//! registered from outside. Tagged literals (`#uuid`, `#inst`) are represented
//! by the generic [`Value::Tagged`] form so the *syntax* is supported without
//! the kernel knowing what a uuid or instant means.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use super::error::{LdError, LdResult};

/// A λδ value.
#[derive(Clone)]
pub enum Value {
    /// Absence; the only "empty". Falsy.
    Nil,
    /// `true` / `false`. `false` is the only other falsy value.
    Bool(bool),
    /// 64-bit signed integer.
    Int(i64),
    /// 64-bit float. `/` and decimal literals produce floats.
    Float(f64),
    /// UTF-8 string.
    Str(Rc<str>),
    /// An identifier — looked up in scope when evaluated.
    Symbol(Rc<str>),
    /// `:like-this` — self-evaluating; the canonical map key. Also callable as
    /// a function that looks itself up in a map (`(:k m)`).
    Keyword(Rc<str>),
    /// A linked sequence — the form of *code*. `(f a b)` is a call.
    List(Rc<Vec<Value>>),
    /// An indexed sequence — the form of *data*.
    Vector(Rc<Vec<Value>>),
    /// `#{…}` — unordered unique members (stored in insertion order for
    /// deterministic printing; membership uses value equality).
    Set(Rc<Vec<Value>>),
    /// `{:k v …}` — keyword→value (insertion-ordered, unique keys).
    Map(Rc<Vec<(Value, Value)>>),
    /// `#tag v` — an extensible tagged literal (`#uuid`, `#inst`, …). The kernel
    /// keeps the form; hosts assign meaning.
    Tagged { tag: Rc<str>, value: Rc<Value> },
    /// A user closure created by `fn`.
    Fn(Rc<Closure>),
    /// A native function registered into the kernel (kernel builtin or host
    /// binding — the seam is the same).
    Builtin(Rc<Builtin>),
}

/// A user-defined closure: parameters, an optional `& rest` param, a body
/// (evaluated as an implicit `do`), and the environment it closed over.
pub struct Closure {
    pub name: Option<Rc<str>>,
    pub params: Vec<Rc<str>>,
    pub rest: Option<Rc<str>>,
    pub body: Vec<Value>,
    pub env: Env,
}

/// The concrete implementation of a native function. Receives the interpreter
/// (for `apply`, budget, and — for host builtins — captured host state) and the
/// already-evaluated argument slice.
pub type BuiltinImpl = dyn Fn(&mut super::Interp, &[Value]) -> LdResult<Value>;

/// A native function plus its arity contract and display name. This is the unit
/// registered through the host seam (`Interp::register_builtin`).
pub struct Builtin {
    pub name: Rc<str>,
    /// Minimum argument count.
    pub min_arity: usize,
    /// Maximum argument count; `None` means variadic.
    pub max_arity: Option<usize>,
    pub func: Box<BuiltinImpl>,
}

impl Builtin {
    /// Check an argument count against this builtin's arity, producing a precise
    /// [`LdError::Arity`] on mismatch.
    pub fn check_arity(&self, got: usize) -> LdResult<()> {
        let ok = got >= self.min_arity && self.max_arity.is_none_or(|m| got <= m);
        if ok {
            return Ok(());
        }
        let expected = match self.max_arity {
            Some(m) if m == self.min_arity => format!("{}", self.min_arity),
            Some(m) => format!("{}..{}", self.min_arity, m),
            None => format!("at least {}", self.min_arity),
        };
        Err(LdError::Arity {
            name: self.name.to_string(),
            expected,
            got,
        })
    }
}

// ── Truthiness ───────────────────────────────────────────────────────────────

impl Value {
    /// Spec §2: only `nil` and `false` are falsy. Everything else — including
    /// `0` and `""` — is truthy.
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    /// A short type name for diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Symbol(_) => "symbol",
            Value::Keyword(_) => "keyword",
            Value::List(_) => "list",
            Value::Vector(_) => "vector",
            Value::Set(_) => "set",
            Value::Map(_) => "map",
            Value::Tagged { .. } => "tagged",
            Value::Fn(_) => "function",
            Value::Builtin(_) => "function",
        }
    }

    // Ergonomic constructors used across the kernel and tests.
    pub fn str<S: Into<Rc<str>>>(s: S) -> Value {
        Value::Str(s.into())
    }
    pub fn sym<S: Into<Rc<str>>>(s: S) -> Value {
        Value::Symbol(s.into())
    }
    pub fn kw<S: Into<Rc<str>>>(s: S) -> Value {
        Value::Keyword(s.into())
    }
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }
    pub fn vector(items: Vec<Value>) -> Value {
        Value::Vector(Rc::new(items))
    }
}

// ── Equality ─────────────────────────────────────────────────────────────────

/// λδ value equality, used by `=`, set membership, and map keys.
///
/// One deliberate resolution the spec left open: numbers compare **across**
/// int/float (`(= 1 1.0)` → true), because notebook attributes round-trip
/// through JSON where `2` and `2.0` are indistinguishable in intent — strict
/// variant equality there would be a quiet footgun. Lists and vectors compare
/// element-wise regardless of which sequence kind they are (Clojure-style
/// sequential equality). Functions are never equal.
pub fn value_eq(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Nil, Nil) => true,
        (Bool(x), Bool(y)) => x == y,
        (Int(x), Int(y)) => x == y,
        (Float(x), Float(y)) => x == y,
        (Int(x), Float(y)) | (Float(y), Int(x)) => (*x as f64) == *y,
        (Str(x), Str(y)) => x == y,
        (Symbol(x), Symbol(y)) => x == y,
        (Keyword(x), Keyword(y)) => x == y,
        (List(x), List(y))
        | (Vector(x), Vector(y))
        | (List(x), Vector(y))
        | (Vector(x), List(y)) => seq_eq(x, y),
        (Set(x), Set(y)) => x.len() == y.len() && x.iter().all(|m| set_contains(y, m)),
        (Map(x), Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| map_lookup(y, k).is_some_and(|w| value_eq(v, w)))
        }
        (Tagged { tag: t1, value: v1 }, Tagged { tag: t2, value: v2 }) => {
            t1 == t2 && value_eq(v1, v2)
        }
        _ => false,
    }
}

fn seq_eq(x: &[Value], y: &[Value]) -> bool {
    x.len() == y.len() && x.iter().zip(y).all(|(a, b)| value_eq(a, b))
}

/// Is `x` a member of set `members` (by value equality)?
pub fn set_contains(members: &[Value], x: &Value) -> bool {
    members.iter().any(|m| value_eq(m, x))
}

/// Look up `key` in an insertion-ordered map's pairs (by value equality).
pub fn map_lookup<'a>(pairs: &'a [(Value, Value)], key: &Value) -> Option<&'a Value> {
    pairs.iter().find(|(k, _)| value_eq(k, key)).map(|(_, v)| v)
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        value_eq(self, other)
    }
}

// ── Display (a reader-compatible printer) ────────────────────────────────────

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => f.write_str("nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => write!(f, "{}", format_float(*x)),
            Value::Str(s) => write!(f, "\"{}\"", escape_str(s)),
            Value::Symbol(s) => f.write_str(s),
            Value::Keyword(s) => write!(f, ":{s}"),
            Value::List(items) => write_seq(f, "(", items, ")"),
            Value::Vector(items) => write_seq(f, "[", items, "]"),
            Value::Set(items) => write_seq(f, "#{", items, "}"),
            Value::Map(pairs) => {
                f.write_str("{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{k} {v}")?;
                }
                f.write_str("}")
            }
            Value::Tagged { tag, value } => write!(f, "#{tag} {value}"),
            Value::Fn(c) => match &c.name {
                Some(n) => write!(f, "#<fn {n}>"),
                None => f.write_str("#<fn>"),
            },
            Value::Builtin(b) => write!(f, "#<builtin {}>", b.name),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The readable form is also the most useful debug form.
        fmt::Display::fmt(self, f)
    }
}

fn write_seq(f: &mut fmt::Formatter<'_>, open: &str, items: &[Value], close: &str) -> fmt::Result {
    f.write_str(open)?;
    for (i, v) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(" ")?;
        }
        write!(f, "{v}")?;
    }
    f.write_str(close)
}

/// Format a float so it always reads back as a float (never as an int token).
fn format_float(x: f64) -> String {
    if x.is_nan() {
        return "##NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "##Inf" } else { "##-Inf" }.to_string();
    }
    let s = format!("{x}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

// ── Lexical environment ──────────────────────────────────────────────────────

/// A shared, reference-counted scope. Closures capture one of these; a call
/// frame is a fresh child scope whose parent is the closure's captured scope.
pub type Env = Rc<Scope>;

/// One lexical scope: its own bindings plus an optional parent.
pub struct Scope {
    vars: RefCell<HashMap<Rc<str>, Value>>,
    parent: Option<Env>,
}

impl Scope {
    /// A new root (global) scope.
    pub fn root() -> Env {
        Rc::new(Scope {
            vars: RefCell::new(HashMap::new()),
            parent: None,
        })
    }

    /// A new child scope nested under `parent`.
    pub fn child(parent: Env) -> Env {
        Rc::new(Scope {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent),
        })
    }

    /// Resolve `name` through the scope chain, cloning the value out.
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.borrow().get(name) {
            return Some(v.clone());
        }
        self.parent.as_ref().and_then(|p| p.get(name))
    }

    /// Bind `name` in *this* scope (shadowing any outer binding).
    pub fn set(&self, name: Rc<str>, value: Value) {
        self.vars.borrow_mut().insert(name, value);
    }
}
