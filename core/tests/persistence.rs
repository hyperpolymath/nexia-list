// SPDX-License-Identifier: MPL-2.0
//! Persistence fidelity, schema migration and corruption recovery.
//!
//! Every stored notebook — file, IndexedDB autosave, import — enters through
//! [`Notebook::load_json`]. These tests pin its contract:
//!
//! * a notebook this build wrote reloads with **nothing** lost or changed;
//! * a pre-versioning (v1) file migrates to the current schema;
//! * a file from a newer build is refused, not loaded lossily;
//! * damaged-but-recoverable data is repaired and reported;
//! * arbitrary bytes produce a structured error, never a panic.

use nexia_core::lambdadelta_host::eval_computed_fields;
use nexia_core::storage::{JsonStorage, StorageError};
use nexia_core::{LoadError, Note, NoteId, Notebook, Point2D, Storage, CURRENT_SCHEMA_VERSION};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Serialize to a `serde_json::Value`, so comparisons ignore map ordering.
fn as_value(nb: &Notebook) -> Value {
    serde_json::to_value(nb).expect("notebook serializes")
}

/// Serialize, reload, and return the reloaded notebook (asserting the load
/// needed no repair).
fn reload(nb: &Notebook) -> Notebook {
    let text = serde_json::to_string(nb).expect("notebook serializes");
    let (loaded, report) = Notebook::load_json(&text).expect("own output reloads");
    assert!(report.is_clean(), "own output needed repair: {report:?}");
    assert!(!report.migrated, "own output was migrated: {report:?}");
    loaded
}

// ── Generators ───────────────────────────────────────────────────────────

/// A JSON attribute value of modest depth.
fn attr_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| json!(n)),
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(|f| json!(f)),
        "\\PC{0,24}".prop_map(Value::String),
    ];
    leaf.prop_recursive(2, 8, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::hash_map("[a-z]{1,6}", inner, 0..4)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Raw material for one note; links are indices resolved after creation.
#[derive(Debug, Clone)]
struct NoteSpec {
    title: String,
    content: String,
    position: Option<(f64, f64)>,
    size: Option<(f64, f64)>,
    attributes: Vec<(String, Value)>,
    computed: Vec<(String, String)>,
    links: Vec<usize>,
}

fn finite() -> impl Strategy<Value = f64> {
    any::<f64>().prop_filter("finite", |f| f.is_finite())
}

fn note_spec() -> impl Strategy<Value = NoteSpec> {
    (
        "\\PC{0,40}",
        "\\PC{0,200}",
        prop::option::of((finite(), finite())),
        prop::option::of((finite(), finite())),
        prop::collection::vec(("[a-z_]{1,10}", attr_value()), 0..4),
        prop::collection::vec(("[a-z_]{1,10}", "\\PC{0,40}"), 0..3),
        prop::collection::vec(any::<usize>(), 0..5),
    )
        .prop_map(
            |(title, content, position, size, attributes, computed, links)| NoteSpec {
                title,
                content,
                position,
                size,
                attributes,
                computed,
                links,
            },
        )
}

/// Build a notebook through the public API from generated specs.
fn build(name: &str, specs: &[NoteSpec], agents: &[(String, String)]) -> Notebook {
    let mut nb = Notebook::new(name);
    let ids: Vec<NoteId> = specs
        .iter()
        .map(|spec| {
            let mut note = Note::new(spec.title.clone());
            note.content = spec.content.clone();
            if let Some((x, y)) = spec.position {
                note = note.with_position(x, y);
            }
            note.size = spec.size;
            for (k, v) in &spec.attributes {
                note.set_attribute(k.clone(), v.clone());
            }
            for (k, src) in &spec.computed {
                note.set_computed(k.clone(), src.clone());
            }
            nb.add_note(note)
        })
        .collect();
    for (spec, &from) in specs.iter().zip(&ids) {
        for &i in &spec.links {
            let to = ids[i % ids.len()];
            if to != from {
                // Duplicate links are rejected by the API, which is fine.
                let _ = nb.link_notes(from, to);
            }
        }
    }
    for (agent_name, query) in agents {
        nb.add_agent(agent_name.clone(), query.clone());
    }
    nb
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Save → load reproduces the notebook exactly: every field of every
    /// note (unicode text, f64 geometry, nested attributes, computed-field
    /// sources, links), agents, metadata and the backlink index.
    #[test]
    fn round_trip_is_lossless(
        name in "\\PC{0,30}",
        specs in prop::collection::vec(note_spec(), 1..12),
        agents in prop::collection::vec(("\\PC{1,12}", "\\PC{0,20}"), 0..3),
    ) {
        let original = build(&name, &specs, &agents);
        let loaded = reload(&original);
        prop_assert_eq!(as_value(&loaded), as_value(&original));

        for id in original.all_note_ids() {
            let mut want = original.get_backlinks(id);
            let mut got = loaded.get_backlinks(id);
            want.sort();
            got.sort();
            prop_assert_eq!(got, want);
        }

        // A second cycle is a fixed point, down to the byte: output is
        // canonical (sorted), so re-saving never churns the file.
        let once = serde_json::to_string(&loaded).expect("serializes");
        let twice = serde_json::to_string(&reload(&loaded)).expect("serializes");
        prop_assert_eq!(once, twice);
    }

    /// No byte string — random, or a valid notebook cut short — panics the
    /// loader. Unloadable input is a structured `LoadError`.
    #[test]
    fn arbitrary_input_never_panics(garbage in "\\PC{0,400}") {
        let _ = Notebook::load_json(&garbage);
    }

    #[test]
    fn truncated_notebook_is_a_structured_error(
        specs in prop::collection::vec(note_spec(), 1..6),
        cut in any::<prop::sample::Index>(),
    ) {
        let text = serde_json::to_string(&build("T", &specs, &[])).expect("serializes");
        let mut end = cut.index(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        // Every strict prefix of a JSON object is incomplete.
        match Notebook::load_json(&text[..end]) {
            Err(LoadError::Corrupt(_)) => {}
            other => prop_assert!(false, "prefix of len {} gave {:?}", end, other.map(|(_, r)| r)),
        }
    }
}

// ── Migration ────────────────────────────────────────────────────────────

const GOLDEN_V1: &str = include_str!("../../tests/fixtures/notebook.golden.json");

#[test]
fn legacy_file_migrates_to_current_schema() {
    let (nb, report) = Notebook::load_json(GOLDEN_V1).expect("golden loads");
    assert_eq!(report.from_version, 1);
    assert!(report.migrated);
    assert!(report.is_clean(), "{report:?}");
    assert_eq!(nb.schema_version(), CURRENT_SCHEMA_VERSION);

    // Re-saving writes the current version, which then loads without migrating.
    let saved = as_value(&nb);
    assert_eq!(saved["schema_version"], json!(CURRENT_SCHEMA_VERSION));
    let (_, again) = Notebook::load_json(&saved.to_string()).expect("reloads");
    assert!(!again.migrated);
}

#[test]
fn newer_schema_is_refused_not_truncated() {
    let mut doc: Value = serde_json::from_str(GOLDEN_V1).expect("fixture is JSON");
    doc["schema_version"] = json!(CURRENT_SCHEMA_VERSION + 1);
    doc["field_from_the_future"] = json!({"keep": "me"});
    match Notebook::load_json(&doc.to_string()) {
        Err(LoadError::UnsupportedSchema { found, supported }) => {
            assert_eq!(found, CURRENT_SCHEMA_VERSION + 1);
            assert_eq!(supported, CURRENT_SCHEMA_VERSION);
        }
        other => panic!(
            "expected UnsupportedSchema, got {:?}",
            other.map(|(_, r)| r)
        ),
    }
}

// ── Corruption recovery ──────────────────────────────────────────────────

#[test]
fn malformed_documents_are_corrupt_errors() {
    for bad in [
        "",
        "null",
        "[]",
        "{",
        r#"{"name": 7}"#,
        r#"{"notes": "nope", "name": "x"}"#,
        r#"{"schema_version": "two", "notes": {}, "name": "x",
            "created_at": "2026-01-01T00:00:00Z", "modified_at": "2026-01-01T00:00:00Z"}"#,
    ] {
        assert!(
            matches!(Notebook::load_json(bad), Err(LoadError::Corrupt(_))),
            "accepted {bad:?}"
        );
    }
}

/// Hand-assemble a damaged v2 document from raw parts.
fn damaged_doc(notes: Value) -> String {
    json!({
        "schema_version": CURRENT_SCHEMA_VERSION,
        "notes": notes,
        // A stale index that must be ignored.
        "backlinks": {"99999999-9999-4999-8999-999999999999": ["00000000-0000-4000-8000-000000000000"]},
        "name": "Damaged",
        "created_at": "2026-01-01T00:00:00Z",
        "modified_at": "2026-01-01T00:00:00Z",
    })
    .to_string()
}

fn raw_note(id: &str, links: &[&str]) -> Value {
    json!({
        "id": id,
        "title": format!("note {id}"),
        "content": "",
        "created_at": "2026-01-01T00:00:00Z",
        "modified_at": "2026-01-01T00:00:00Z",
        "links": links,
    })
}

const A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const C: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const GHOST: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
/// A valid id that is not the id of the note stored under it.
const WRONG_KEY: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";

fn id(s: &str) -> NoteId {
    NoteId::parse_str(s).expect("valid uuid")
}

#[test]
fn damaged_graph_is_repaired_and_reported() {
    let doc = damaged_doc(json!({
        A: raw_note(A, &[B, B, A, GHOST]),     // duplicate, self-link, dangling
        WRONG_KEY: raw_note(C, &[A]),          // stored under the wrong key
        B: raw_note(B, &[]),
    }));
    let (nb, report) = Notebook::load_json(&doc).expect("repairable");

    assert_eq!(report.duplicate_links, 1);
    assert_eq!(report.self_links, vec![id(A)]);
    assert_eq!(report.dangling_links, vec![(id(A), id(GHOST))]);
    assert_eq!(report.rekeyed_notes, vec![id(C)]);
    assert!(!report.is_clean());

    assert_eq!(nb.len(), 3);
    assert_eq!(nb.get_note(&id(A)).expect("A").links, vec![id(B)]);
    assert!(
        nb.get_note(&id(C)).is_some(),
        "re-keyed note is reachable by id"
    );
    assert_eq!(nb.get_backlinks(&id(B)), vec![id(A)]);
    assert_eq!(nb.get_backlinks(&id(A)), vec![id(C)]);
    assert!(
        nb.get_backlinks(&id("99999999-9999-4999-8999-999999999999"))
            .is_empty(),
        "stale stored index must not survive"
    );

    // Repaired output is clean on the next load.
    let (_, second) =
        Notebook::load_json(&serde_json::to_string(&nb).expect("ser")).expect("reload");
    assert!(second.is_clean(), "{second:?}");
}

#[test]
fn colliding_ids_keep_one_note_deterministically() {
    let doc = damaged_doc(json!({
        A: raw_note(A, &[]),
        WRONG_KEY: raw_note(A, &[]),
    }));
    let (nb, report) = Notebook::load_json(&doc).expect("repairable");
    assert_eq!(nb.len(), 1);
    assert_eq!(report.dropped_duplicate_ids, vec![id(A)]);
}

// ── Non-finite geometry (NaN/Infinity cannot be stored in JSON) ─────────

#[test]
fn non_finite_geometry_never_makes_a_notebook_unloadable() {
    let mut nb = Notebook::new("geo");
    let placed = nb.create_note("placed");
    let bad = nb.create_note("bad");
    nb.get_note_mut(&placed).expect("placed").position = Some(Point2D::new(1.5, -2.0));
    {
        let note = nb.get_note_mut(&bad).expect("bad");
        note.position = Some(Point2D::new(f64::NAN, 1.0));
        note.size = Some((f64::INFINITY, 10.0));
    }
    // Written without the unstorable geometry, so it reloads cleanly.
    let (loaded, report) =
        Notebook::load_json(&serde_json::to_string(&nb).expect("ser")).expect("reloads");
    assert!(report.is_clean(), "{report:?}");
    let bad_note = loaded.get_note(&bad).expect("bad survives");
    assert_eq!((bad_note.position, bad_note.size), (None, None));
    assert_eq!(
        loaded.get_note(&placed).expect("placed").position,
        Some(Point2D::new(1.5, -2.0))
    );
}

#[test]
fn null_coordinates_from_older_builds_are_repaired() {
    let mut note = raw_note(A, &[]);
    note["position"] = json!({"x": null, "y": 3.0});
    note["size"] = json!([null, 4.0]);
    let mut ok = raw_note(B, &[]);
    ok["position"] = json!({"x": 1.0, "y": 2.0});
    let (nb, report) =
        Notebook::load_json(&damaged_doc(json!({ A: note, B: ok }))).expect("repairable");
    assert_eq!(report.invalid_geometry, vec![id(A)]);
    let repaired = nb.get_note(&id(A)).expect("A kept");
    assert_eq!((repaired.position, repaired.size), (None, None));
    assert_eq!(
        nb.get_note(&id(B)).expect("B").position,
        Some(Point2D::new(1.0, 2.0))
    );
}

#[test]
fn lambdadelta_cannot_store_non_finite_geometry() {
    use nexia_core::lambdadelta::{Budget, Interp};
    let mut nb = Notebook::new("ld");
    let n = nb.create_note("n");
    let shared = Rc::new(RefCell::new(nb));
    let mut interp = Interp::new();
    nexia_core::lambdadelta_host::register(&mut interp, shared.clone());
    for program in [
        format!("(move-note! \"{n}\" (* 1e308 10.0) 0)"),
        format!("(resize-note! \"{n}\" 10 (* -1e308 10.0))"),
    ] {
        assert!(
            interp.eval_str(&program, Budget::new()).is_err(),
            "{program}"
        );
    }
    assert!(interp
        .eval_str(&format!("(move-note! \"{n}\" 5 6)"), Budget::new())
        .is_ok());
    drop(interp);
    let note_pos = shared.borrow().get_note(&n).expect("n").position;
    assert_eq!(note_pos, Some(Point2D::new(5.0, 6.0)));
}

// ── Computed fields ──────────────────────────────────────────────────────

#[test]
fn computed_fields_survive_reload_and_evaluate_identically() {
    let mut nb = Notebook::new("fx");
    let target = nb.create_note("Target");
    let source = nb.create_note("Source");
    nb.set_content(&source, "one two three [[Target]]");
    nb.set_computed(&source, "words", "(count (words (content self)))")
        .expect("note exists");
    nb.set_computed(&source, "broken", "(car)")
        .expect("note exists");
    nb.set_computed(&target, "title-len", "(count (title self))")
        .expect("note exists");

    let evaluate = |nb: Notebook, id: NoteId| {
        let shared = Rc::new(RefCell::new(nb));
        eval_computed_fields(shared, &id)
    };
    let before = evaluate(nb.clone(), source);
    let after = evaluate(reload(&nb), source);
    assert_eq!(before, after);

    let names: Vec<&str> = after.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["broken", "words"], "name order, both present");
    assert!(
        after[0].result.is_err(),
        "a failing field reports its error"
    );
    assert_eq!(
        after[1].result,
        Ok("4".to_string()),
        "siblings still evaluate"
    );

    let target_fields = evaluate(reload(&nb), target);
    assert_eq!(target_fields[0].result, Ok("6".to_string()));
}

#[test]
fn computed_field_removal_persists() {
    let mut nb = Notebook::new("fx");
    let n = nb.create_note("N");
    nb.set_computed(&n, "f", "1").expect("exists");
    assert!(nb.remove_computed(&n, "f").expect("exists"));
    assert!(!nb.remove_computed(&n, "f").expect("exists"));
    assert!(reload(&nb).get_note(&n).expect("N").computed.is_empty());
    assert!(nb.set_computed(&id(GHOST), "f", "1").is_err());
}

// ── File storage ─────────────────────────────────────────────────────────

#[test]
fn file_save_is_atomic_and_reports_on_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nb.nexia.json");
    let storage = JsonStorage::new();

    let mut nb = Notebook::new("Disk");
    let a = nb.create_note("A");
    nb.set_computed(&a, "x", "(+ 1 2)").expect("exists");
    storage.save(&nb, &path).expect("save");
    storage.save(&nb, &path).expect("overwrite");

    let leftovers: HashSet<String> = std::fs::read_dir(dir.path())
        .expect("readdir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(leftovers, HashSet::from(["nb.nexia.json".to_string()]));

    let (loaded, report) = storage.load_with_report(&path).expect("load");
    assert!(report.is_clean() && !report.migrated);
    assert_eq!(as_value(&loaded), as_value(&nb));

    std::fs::write(&path, "{\"notes\":").expect("corrupt it");
    assert!(matches!(
        storage.load(&path),
        Err(StorageError::Load(LoadError::Corrupt(_)))
    ));
}
