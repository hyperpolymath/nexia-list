// SPDX-License-Identifier: MPL-2.0
//! The **notebook host** for LambdaDelta (spec §4, §7).
//!
//! This is where λδ first *touches a note*. It lives **outside** the kernel
//! (`core/src/lambdadelta/`) on purpose: the kernel knows nothing about notes,
//! and this module registers the notebook builtins *into* a kernel through the
//! seam ([`Interp::register_builtin`]). Nexia-List is simply the first host — an
//! embedder with a different data model would write its own host just like this.
//!
//! Two halves:
//! 1. **Bridge** — a [`Note`] becomes an *immutable snapshot map* (spec §2), and
//!    attribute JSON maps losslessly to/from λδ values.
//! 2. **Builtins** — pure *readers* (`notes`, `attr`, `search`, …) and
//!    `!`-suffixed *mutators* (`set-attr!`, `create-note!`, `link!`, …), each
//!    closing over a shared `Rc<RefCell<Notebook>>`.
//!
//! Evaluation-context gating (formulas pure, only actions may mutate — spec §5)
//! is a higher layer's job; this module makes the whole surface *available*.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::lambdadelta::{Budget, Interp, LdError, LdResult, Value};
use crate::note::Point2D;
use crate::notebook::Notebook;

/// Register every notebook builtin into `interp`, each sharing `nb`.
///
/// ```
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use nexia_core::notebook::Notebook;
/// use nexia_core::lambdadelta::{Interp, Budget, Value};
///
/// let mut nb = Notebook::new("demo");
/// nb.create_note("Alpha");
/// let shared = Rc::new(RefCell::new(nb));
///
/// let mut interp = Interp::new();
/// nexia_core::lambdadelta_host::register(&mut interp, shared.clone());
///
/// let out = interp.eval_str("(count (notes))", Budget::new()).unwrap();
/// assert_eq!(out, Value::Int(1));
/// ```
pub fn register(interp: &mut Interp, nb: Rc<RefCell<Notebook>>) {
    register_readers(interp, &nb);
    register_mutators(interp, &nb);
}

/// Register only the pure reader builtins — the surface a **formula** or
/// **agent-predicate** context is allowed (spec §5).
pub fn register_readers(interp: &mut Interp, nb: &Rc<RefCell<Notebook>>) {
    reader(interp, nb, "notes", 0, Some(0), bi_notes);
    reader(interp, nb, "note", 1, Some(1), bi_note);
    reader(interp, nb, "title", 1, Some(1), bi_title);
    reader(interp, nb, "content", 1, Some(1), bi_content);
    reader(interp, nb, "attrs", 1, Some(1), bi_attrs);
    reader(interp, nb, "links", 1, Some(1), bi_links);
    reader(interp, nb, "backlinks", 1, Some(1), bi_backlinks);
    reader(interp, nb, "position", 1, Some(1), bi_position);
    reader(interp, nb, "attr", 2, Some(2), bi_attr);
    reader(interp, nb, "search", 1, Some(1), bi_search);
    reader(interp, nb, "resolve-title", 1, Some(1), bi_resolve_title);
    reader(interp, nb, "agents", 0, Some(0), bi_agents);
    reader(interp, nb, "run-agent", 1, Some(1), bi_run_agent);
}

/// Register the `!`-suffixed mutators — permitted only in **action** contexts
/// (on-create / agent-action / stamp; spec §5).
pub fn register_mutators(interp: &mut Interp, nb: &Rc<RefCell<Notebook>>) {
    mutator(interp, nb, "create-note!", 1, Some(3), bi_create_note);
    mutator(interp, nb, "set-title!", 2, Some(2), bi_set_title);
    mutator(interp, nb, "set-content!", 2, Some(2), bi_set_content);
    mutator(interp, nb, "set-attr!", 3, Some(3), bi_set_attr);
    mutator(interp, nb, "remove-attr!", 2, Some(2), bi_remove_attr);
    mutator(interp, nb, "move-note!", 3, Some(3), bi_move_note);
    mutator(interp, nb, "resize-note!", 3, Some(3), bi_resize_note);
    mutator(interp, nb, "link!", 2, Some(2), bi_link);
    mutator(interp, nb, "unlink!", 2, Some(2), bi_unlink);
    mutator(interp, nb, "delete-note!", 1, Some(1), bi_delete_note);
}

/// Evaluate a **formula** (spec §5): a pure expression with `self` bound to a
/// note's snapshot map and only the *reader* builtins in scope — mutators are
/// deliberately absent, so a formula can never change the notebook. This is the
/// L1 surface: `(count (words (content self)))`, `(= (attr self :status) "todo")`.
pub fn eval_formula(
    nb: Rc<RefCell<Notebook>>,
    self_id: &Uuid,
    src: &str,
    budget: Budget,
) -> LdResult<Value> {
    let mut interp = Interp::new();
    register_readers(&mut interp, &nb);
    let self_val = note_to_value(&nb.borrow(), self_id).unwrap_or(Value::Nil);
    interp.global.set(Rc::from("self"), self_val);
    interp.eval_str(src, budget)
}

type ReadFn = fn(&Notebook, &[Value]) -> LdResult<Value>;
type MutFn = fn(&mut Notebook, &[Value]) -> LdResult<Value>;

fn reader(
    interp: &mut Interp,
    nb: &Rc<RefCell<Notebook>>,
    name: &str,
    min: usize,
    max: Option<usize>,
    f: ReadFn,
) {
    let n = nb.clone();
    interp.register_builtin(name, min, max, move |_i, a| {
        let g = n.borrow();
        f(&g, a)
    });
}

fn mutator(
    interp: &mut Interp,
    nb: &Rc<RefCell<Notebook>>,
    name: &str,
    min: usize,
    max: Option<usize>,
    f: MutFn,
) {
    let n = nb.clone();
    interp.register_builtin(name, min, max, move |_i, a| {
        let mut g = n.borrow_mut();
        f(&mut g, a)
    });
}

// ── Bridge: Note → immutable snapshot map (spec §2) ──────────────────────────

/// Build the immutable snapshot map for a note (spec §2).
pub fn note_to_value(nb: &Notebook, id: &Uuid) -> Option<Value> {
    let note = nb.get_note(id)?;
    let backlinks = nb.get_backlinks(id);
    let attrs: Vec<(Value, Value)> = note
        .attributes
        .iter()
        .map(|(k, v)| (Value::kw(k.as_str()), json_to_value(v)))
        .collect();
    let links: Vec<Value> = note.links.iter().map(uuid_value).collect();
    let backs: Vec<Value> = backlinks.iter().map(uuid_value).collect();
    let ty = note
        .attributes
        .get("type")
        .map(json_to_value)
        .unwrap_or(Value::Nil);
    let pairs = vec![
        (Value::kw("id"), uuid_value(&note.id)),
        (Value::kw("title"), Value::str(note.title.as_str())),
        (Value::kw("content"), Value::str(note.content.as_str())),
        (Value::kw("attrs"), Value::Map(Rc::new(attrs))),
        (Value::kw("links"), Value::vector(links)),
        (Value::kw("backlinks"), Value::vector(backs)),
        (Value::kw("position"), point_value(note.position)),
        (Value::kw("size"), size_value(note.size)),
        (
            Value::kw("prototype"),
            note.prototype.map_or(Value::Nil, |p| uuid_value(&p)),
        ),
        (Value::kw("type"), ty),
        (Value::kw("created-at"), inst_value(&note.created_at)),
        (Value::kw("modified-at"), inst_value(&note.modified_at)),
    ];
    Some(Value::Map(Rc::new(pairs)))
}

fn uuid_value(id: &Uuid) -> Value {
    Value::Tagged {
        tag: Rc::from("uuid"),
        value: Rc::new(Value::str(id.to_string())),
    }
}

fn inst_value(dt: &DateTime<Utc>) -> Value {
    Value::Tagged {
        tag: Rc::from("inst"),
        value: Rc::new(Value::str(dt.to_rfc3339())),
    }
}

fn point_value(p: Option<Point2D>) -> Value {
    match p {
        Some(p) => Value::vector(vec![Value::Float(p.x), Value::Float(p.y)]),
        None => Value::Nil,
    }
}

fn size_value(s: Option<(f64, f64)>) -> Value {
    match s {
        Some((w, h)) => Value::vector(vec![Value::Float(w), Value::Float(h)]),
        None => Value::Nil,
    }
}

/// JSON attribute value → λδ value (spec §2 mapping; object keys become keywords).
fn json_to_value(j: &serde_json::Value) -> Value {
    use serde_json::Value as J;
    match j {
        J::Null => Value::Nil,
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Float(n.as_f64().unwrap_or(0.0)),
        },
        J::String(s) => Value::str(s.as_str()),
        J::Array(a) => Value::vector(a.iter().map(json_to_value).collect()),
        J::Object(o) => {
            let pairs = o
                .iter()
                .map(|(k, v)| (Value::kw(k.as_str()), json_to_value(v)))
                .collect();
            Value::Map(Rc::new(pairs))
        }
    }
}

/// λδ value → JSON attribute value. Total. Lossy edges (documented): keywords
/// and symbols become strings, sets become arrays, a tagged value stores its
/// inner value (e.g. a `#uuid` becomes its string), and functions become null.
fn value_to_json(v: &Value) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        Value::Nil => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(n) => J::Number((*n).into()),
        Value::Float(x) => serde_json::Number::from_f64(*x).map_or(J::Null, J::Number),
        Value::Str(s) => J::String(s.to_string()),
        Value::Symbol(s) | Value::Keyword(s) => J::String(s.to_string()),
        Value::List(xs) | Value::Vector(xs) | Value::Set(xs) => {
            J::Array(xs.iter().map(value_to_json).collect())
        }
        Value::Map(pairs) => {
            let mut o = serde_json::Map::new();
            for (k, val) in pairs.iter() {
                o.insert(json_key(k), value_to_json(val));
            }
            J::Object(o)
        }
        Value::Tagged { value, .. } => value_to_json(value),
        Value::Fn(_) | Value::Builtin(_) => J::Null,
    }
}

fn json_key(k: &Value) -> String {
    match k {
        Value::Keyword(s) | Value::Symbol(s) | Value::Str(s) => s.to_string(),
        other => other.to_string(),
    }
}

// ── Argument helpers ─────────────────────────────────────────────────────────

/// Resolve a note id from an argument: a `#uuid` tagged value, a uuid string,
/// or a note map (from which `:id` is read).
fn arg_id(v: &Value) -> LdResult<Uuid> {
    match v {
        Value::Tagged { tag, value } if tag.as_ref() == "uuid" => parse_uuid(value),
        Value::Str(_) => parse_uuid(v),
        Value::Map(_) => match map_get(v, "id") {
            Some(idv) => arg_id(idv),
            None => Err(id_err(v)),
        },
        _ => Err(id_err(v)),
    }
}

fn parse_uuid(v: &Value) -> LdResult<Uuid> {
    match v {
        Value::Str(s) => Uuid::parse_str(s).map_err(|_| id_err(v)),
        other => Err(id_err(other)),
    }
}

/// Look up a keyword key in a λδ map value.
fn map_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Map(pairs) => {
            let k = Value::kw(key);
            pairs.iter().find(|(pk, _)| *pk == k).map(|(_, val)| val)
        }
        _ => None,
    }
}

/// A note field accessor that works on either a note map (e.g. `self`) or an id.
fn note_field(nb: &Notebook, arg: &Value, key: &str) -> LdResult<Value> {
    if let Value::Map(_) = arg {
        return Ok(map_get(arg, key).cloned().unwrap_or(Value::Nil));
    }
    let id = arg_id(arg)?;
    match note_to_value(nb, &id) {
        Some(m) => Ok(map_get(&m, key).cloned().unwrap_or(Value::Nil)),
        None => Ok(Value::Nil),
    }
}

fn want_str(v: &Value) -> LdResult<&str> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(type_err("string", other)),
    }
}

fn want_f64(v: &Value) -> LdResult<f64> {
    match v {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(x) => Ok(*x),
        other => Err(type_err("number", other)),
    }
}

fn attr_key(v: &Value) -> LdResult<String> {
    match v {
        Value::Keyword(s) | Value::Str(s) | Value::Symbol(s) => Ok(s.to_string()),
        other => Err(type_err("keyword or string", other)),
    }
}

fn type_err(expected: &str, got: &Value) -> LdError {
    LdError::Type {
        op: "notebook".to_string(),
        expected: expected.to_string(),
        got: got.type_name().to_string(),
    }
}

fn id_err(v: &Value) -> LdError {
    LdError::Type {
        op: "note-id".to_string(),
        expected: "a note id or note map".to_string(),
        got: v.type_name().to_string(),
    }
}

fn not_found(id: &Uuid) -> LdError {
    LdError::User(format!("note not found: {id}"))
}

// ── Readers ──────────────────────────────────────────────────────────────────

fn bi_notes(nb: &Notebook, _a: &[Value]) -> LdResult<Value> {
    let ids: Vec<Uuid> = nb.all_note_ids().copied().collect();
    Ok(Value::vector(
        ids.iter().filter_map(|id| note_to_value(nb, id)).collect(),
    ))
}

fn bi_note(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_title(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "title")
}
fn bi_content(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "content")
}
fn bi_attrs(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "attrs")
}
fn bi_links(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "links")
}
fn bi_backlinks(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "backlinks")
}
fn bi_position(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    note_field(nb, &a[0], "position")
}

fn bi_attr(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    let attrs = note_field(nb, &a[0], "attrs")?;
    let key = attr_key(&a[1])?;
    Ok(map_get(&attrs, &key).cloned().unwrap_or(Value::Nil))
}

fn bi_search(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    let q = want_str(&a[0])?;
    let ids: Vec<Uuid> = nb.search(q).iter().map(|n| n.id).collect();
    Ok(Value::vector(
        ids.iter().filter_map(|id| note_to_value(nb, id)).collect(),
    ))
}

fn bi_resolve_title(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    let s = want_str(&a[0])?.to_lowercase();
    let found = nb
        .all_notes()
        .find(|n| n.title.to_lowercase() == s)
        .map(|n| uuid_value(&n.id));
    Ok(found.unwrap_or(Value::Nil))
}

fn bi_agents(nb: &Notebook, _a: &[Value]) -> LdResult<Value> {
    let out = nb
        .agents()
        .iter()
        .map(|ag| {
            Value::Map(Rc::new(vec![
                (Value::kw("id"), uuid_value(&ag.id)),
                (Value::kw("name"), Value::str(ag.name.as_str())),
                (Value::kw("query"), Value::str(ag.query.as_str())),
            ]))
        })
        .collect();
    Ok(Value::vector(out))
}

fn bi_run_agent(nb: &Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let ids = nb.run_agent(&id);
    Ok(Value::vector(
        ids.iter().filter_map(|id| note_to_value(nb, id)).collect(),
    ))
}

// ── Mutators ─────────────────────────────────────────────────────────────────

fn bi_create_note(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let title = want_str(&a[0])?.to_string();
    let id = nb.create_note(title);
    if a.len() == 3 {
        let x = want_f64(&a[1])?;
        let y = want_f64(&a[2])?;
        if let Some(note) = nb.get_note_mut(&id) {
            note.position = Some(Point2D::new(x, y));
        }
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_set_title(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let s = want_str(&a[1])?.to_string();
    match nb.get_note_mut(&id) {
        Some(note) => {
            note.title = s;
            note.touch();
        }
        None => return Err(not_found(&id)),
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_set_content(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let s = want_str(&a[1])?.to_string();
    if nb.get_note(&id).is_none() {
        return Err(not_found(&id));
    }
    nb.set_content(&id, s); // also derives [[wiki-links]]
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_set_attr(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let key = attr_key(&a[1])?;
    let jv = value_to_json(&a[2]);
    match nb.get_note_mut(&id) {
        Some(note) => note.set_attribute(key, jv),
        None => return Err(not_found(&id)),
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_remove_attr(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let key = attr_key(&a[1])?;
    match nb.get_note_mut(&id) {
        Some(note) => {
            note.attributes.remove(&key);
            note.touch();
        }
        None => return Err(not_found(&id)),
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_move_note(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let x = want_f64(&a[1])?;
    let y = want_f64(&a[2])?;
    match nb.get_note_mut(&id) {
        Some(note) => {
            note.position = Some(Point2D::new(x, y));
            note.touch();
        }
        None => return Err(not_found(&id)),
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_resize_note(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    let w = want_f64(&a[1])?;
    let h = want_f64(&a[2])?;
    match nb.get_note_mut(&id) {
        Some(note) => {
            note.size = Some((w, h));
            note.touch();
        }
        None => return Err(not_found(&id)),
    }
    Ok(note_to_value(nb, &id).unwrap_or(Value::Nil))
}

fn bi_link(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let from = arg_id(&a[0])?;
    let to = arg_id(&a[1])?;
    nb.link_notes(from, to)
        .map_err(|e| LdError::User(e.to_string()))?;
    Ok(note_to_value(nb, &from).unwrap_or(Value::Nil))
}

fn bi_unlink(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let from = arg_id(&a[0])?;
    let to = arg_id(&a[1])?;
    nb.unlink_notes(from, to)
        .map_err(|e| LdError::User(e.to_string()))?;
    Ok(note_to_value(nb, &from).unwrap_or(Value::Nil))
}

fn bi_delete_note(nb: &mut Notebook, a: &[Value]) -> LdResult<Value> {
    let id = arg_id(&a[0])?;
    // Snapshot before removal so the caller gets the deleted note back.
    let snap = note_to_value(nb, &id).unwrap_or(Value::Nil);
    nb.remove_note(&id);
    Ok(snap)
}

#[cfg(test)]
mod tests;
