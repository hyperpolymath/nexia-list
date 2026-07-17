// SPDX-License-Identifier: MPL-2.0
//! Host-binding tests: λδ reading and mutating a real notebook. Assertions
//! compare whole `Result`s and never unwrap/panic.

use std::cell::RefCell;
use std::rc::Rc;

use uuid::Uuid;

use super::register;
use crate::lambdadelta::{Budget, Interp, LdError, Value};
use crate::notebook::Notebook;

/// A notebook with "Alpha" (content "hello world foo bar") and "Beta", plus an
/// interpreter with the notebook host installed. Returns the shared handle so
/// tests can inspect the underlying notebook too.
fn setup() -> (Rc<RefCell<Notebook>>, Interp) {
    let mut nb = Notebook::new("t");
    let a = nb.create_note("Alpha");
    if let Some(n) = nb.get_note_mut(&a) {
        n.content = "hello world foo bar".into();
    }
    nb.create_note("Beta");
    let shared = Rc::new(RefCell::new(nb));
    let mut interp = Interp::new();
    register(&mut interp, shared.clone());
    (shared, interp)
}

fn id_of(nb: &Rc<RefCell<Notebook>>, title: &str) -> Option<Uuid> {
    nb.borrow()
        .all_notes()
        .find(|n| n.title == title)
        .map(|n| n.id)
}

fn eval(i: &mut Interp, src: &str) -> Result<Value, LdError> {
    i.eval_str(src, Budget::new())
}

#[test]
fn reads_notes_and_fields() {
    let (_nb, mut i) = setup();
    assert_eq!(eval(&mut i, "(count (notes))"), Ok(Value::Int(2)));
    assert_eq!(
        eval(&mut i, "(title (note (resolve-title \"Alpha\")))"),
        Ok(Value::str("Alpha"))
    );
    assert_eq!(
        eval(&mut i, "(:title (note (resolve-title \"Beta\")))"),
        Ok(Value::str("Beta"))
    );
    assert_eq!(eval(&mut i, "(resolve-title \"Ghost\")"), Ok(Value::Nil));
}

#[test]
fn formula_over_a_note() {
    // The canonical L1 example: word count of a note's content.
    let (_nb, mut i) = setup();
    assert_eq!(
        eval(
            &mut i,
            "(count (words (content (note (resolve-title \"Alpha\")))))"
        ),
        Ok(Value::Int(4))
    );
}

#[test]
fn search_returns_note_maps() {
    let (_nb, mut i) = setup();
    assert_eq!(
        eval(&mut i, "(count (search \"Alpha\"))"),
        Ok(Value::Int(1))
    );
    // A content-only match.
    assert_eq!(
        eval(&mut i, "(count (search \"world\"))"),
        Ok(Value::Int(1))
    );
}

#[test]
fn set_attr_mutates_the_notebook() {
    let (nb, mut i) = setup();
    assert!(eval(
        &mut i,
        "(set-attr! (resolve-title \"Alpha\") :status \"todo\")"
    )
    .is_ok());
    // Visible back through λδ:
    assert_eq!(
        eval(&mut i, "(attr (note (resolve-title \"Alpha\")) :status)"),
        Ok(Value::str("todo"))
    );
    // And in the underlying notebook:
    let a = id_of(&nb, "Alpha");
    assert!(a.is_some());
    if let Some(a) = a {
        let stored = nb
            .borrow()
            .get_note(&a)
            .and_then(|n| n.get_attribute("status"))
            .cloned();
        assert_eq!(stored, Some(serde_json::json!("todo")));
    }
}

#[test]
fn create_note_grows_the_notebook() {
    let (nb, mut i) = setup();
    assert!(eval(&mut i, "(create-note! \"Gamma\")").is_ok());
    assert_eq!(eval(&mut i, "(count (notes))"), Ok(Value::Int(3)));
    assert!(id_of(&nb, "Gamma").is_some());
}

#[test]
fn link_updates_topology() {
    let (_nb, mut i) = setup();
    assert!(eval(
        &mut i,
        "(link! (resolve-title \"Alpha\") (resolve-title \"Beta\"))"
    )
    .is_ok());
    assert_eq!(
        eval(&mut i, "(count (links (note (resolve-title \"Alpha\"))))"),
        Ok(Value::Int(1))
    );
    assert_eq!(
        eval(
            &mut i,
            "(count (backlinks (note (resolve-title \"Beta\"))))"
        ),
        Ok(Value::Int(1))
    );
}

#[test]
fn set_content_derives_wikilinks() {
    let (_nb, mut i) = setup();
    assert!(eval(
        &mut i,
        "(set-content! (resolve-title \"Alpha\") \"see [[Beta]]\")"
    )
    .is_ok());
    assert_eq!(
        eval(&mut i, "(count (links (note (resolve-title \"Alpha\"))))"),
        Ok(Value::Int(1))
    );
}

#[test]
fn note_map_is_usable_as_self() {
    // A note map (as `self` would be in a formula) supports the accessors and
    // keyword lookup directly, with no notebook round-trip.
    let (_nb, mut i) = setup();
    assert_eq!(
        eval(
            &mut i,
            "(let [n (note (resolve-title \"Alpha\"))] [(title n) (:title n)])"
        ),
        Ok(Value::vector(vec![
            Value::str("Alpha"),
            Value::str("Alpha")
        ]))
    );
}

#[test]
fn mutator_on_unknown_id_errors() {
    let (_nb, mut i) = setup();
    let r = eval(
        &mut i,
        "(set-title! #uuid \"00000000-0000-0000-0000-000000000000\" \"x\")",
    );
    assert!(matches!(r, Err(LdError::User(_))));
}

#[test]
fn formula_binds_self_and_is_read_only() {
    let (nb, _i) = setup();
    let a = id_of(&nb, "Alpha");
    assert!(a.is_some());
    let Some(a) = a else { return };

    // `self` is bound to the note; readers are in scope. The canonical L1 fx.
    assert_eq!(
        super::eval_formula(
            nb.clone(),
            &a,
            "(count (words (content self)))",
            Budget::new()
        ),
        Ok(Value::Int(4))
    );
    assert_eq!(
        super::eval_formula(nb.clone(), &a, "(:title self)", Budget::new()),
        Ok(Value::str("Alpha"))
    );

    // Mutators are deliberately absent — a formula cannot change the notebook.
    let m = super::eval_formula(
        nb.clone(),
        &a,
        "(set-title! (:id self) \"X\")",
        Budget::new(),
    );
    assert!(matches!(m, Err(LdError::Unbound(_))));
    assert_eq!(id_of(&nb, "Alpha"), Some(a)); // title unchanged
}
