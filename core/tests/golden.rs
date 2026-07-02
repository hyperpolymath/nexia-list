// SPDX-License-Identifier: MPL-2.0
//! Golden-fixture contract test.
//!
//! Guards the on-disk JSON format shared with the web UI: the same fixture
//! is decoded by ui/tests/contract.test.js through the WASM bindings. If a
//! serde shape change breaks either side, CI fails on both.

use nexia_core::{NoteId, Notebook};

const FIXTURE: &str = include_str!("../../tests/fixtures/notebook.golden.json");

fn ids() -> (NoteId, NoteId) {
    (
        NoteId::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
        NoteId::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
    )
}

#[test]
fn golden_fixture_loads_with_rebuilt_backlinks() {
    let mut nb: Notebook = serde_json::from_str(FIXTURE).unwrap();
    nb.rebuild_backlinks();
    let (alpha, beta) = ids();

    assert_eq!(nb.len(), 2);
    assert_eq!(nb.name, "Golden");
    let alpha_note = nb.get_note(&alpha).unwrap();
    assert_eq!(alpha_note.title, "Alpha");
    assert!(alpha_note.links_to(&beta));
    assert_eq!(alpha_note.position.map(|p| (p.x, p.y)), Some((10.0, 20.0)));
    assert_eq!(alpha_note.size, Some((200.0, 150.0)));
    assert_eq!(
        alpha_note.get_attribute("status"),
        Some(&serde_json::json!("todo"))
    );

    // The fixture intentionally omits `backlinks`: loading must rebuild it.
    assert_eq!(nb.get_backlinks(&beta), vec![alpha]);
    assert!(nb.get_backlinks(&alpha).is_empty());

    // Search reaches only the matching note.
    assert_eq!(nb.search("alpha").len(), 1);
}

#[test]
fn golden_fixture_roundtrips() {
    let mut nb: Notebook = serde_json::from_str(FIXTURE).unwrap();
    nb.rebuild_backlinks();
    let (alpha, beta) = ids();

    let json = serde_json::to_string(&nb).unwrap();
    let nb2: Notebook = serde_json::from_str(&json).unwrap();

    assert_eq!(nb2.len(), 2);
    assert_eq!(nb2.name, "Golden");
    assert!(nb2.get_note(&alpha).unwrap().links_to(&beta));
    assert_eq!(
        nb2.get_note(&beta).unwrap().created_at,
        nb.get_note(&beta).unwrap().created_at
    );
}
