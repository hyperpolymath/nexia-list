// SPDX-License-Identifier: MPL-2.0
//! Property tests for the notebook's structural invariants.

use nexia_core::{NoteId, Notebook};
use proptest::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
enum Op {
    Create,
    Link(usize, usize),
    Unlink(usize, usize),
    Remove(usize),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        2 => Just(Op::Create),
        3 => (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Op::Link(a, b)),
        1 => (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Op::Unlink(a, b)),
        1 => any::<usize>().prop_map(Op::Remove),
    ]
}

fn pick(ids: &[NoteId], index: usize) -> Option<NoteId> {
    if ids.is_empty() {
        None
    } else {
        Some(ids[index % ids.len()])
    }
}

/// Validate the notebook's load-bearing graph invariant over both live and
/// formerly-live ids. Checking removed ids matters: a stale reverse-index key
/// is invisible if we inspect only the current note domain.
fn structural_violation(nb: &Notebook, known_ids: &[NoteId]) -> Option<String> {
    for id in known_ids {
        let expected: HashSet<NoteId> = nb
            .all_notes()
            .filter(|n| n.links_to(id))
            .map(|n| n.id)
            .collect();
        let actual: HashSet<NoteId> = nb.get_backlinks(id).into_iter().collect();
        if expected != actual {
            return Some(format!(
                "backlink index diverged for {id}: expected {expected:?}, got {actual:?}"
            ));
        }
    }

    for note in nb.all_notes() {
        let unique: HashSet<NoteId> = note.links.iter().copied().collect();
        if unique.len() != note.links.len() {
            return Some(format!("duplicate outgoing link on {}", note.id));
        }
        if note.links.contains(&note.id) {
            return Some(format!("self-link on {}", note.id));
        }
        for target in &note.links {
            if nb.get_note(target).is_none() {
                return Some(format!("dangling link from {} to {target}", note.id));
            }
        }
    }

    None
}

/// Serialized persistent state with the derived backlink index removed. The
/// index is compared separately as sets because its JSON array order is not
/// semantically meaningful.
fn persistent_value(nb: &Notebook) -> serde_json::Value {
    let mut value = serde_json::to_value(nb).unwrap();
    if let serde_json::Value::Object(object) = &mut value {
        object.remove("backlinks");
    }
    value
}

proptest! {
    /// The persistence parser is total over generated arbitrary byte strings:
    /// malformed UTF-8/JSON is an error, never a process panic.
    #[test]
    fn notebook_json_parser_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let _ = serde_json::from_slice::<Notebook>(&bytes);
    }

    /// After any sequence of operations, the backlinks index is exactly the
    /// inverse of the union of the notes' outgoing links.
    #[test]
    fn backlinks_are_exact_inverse(ops in prop::collection::vec(op_strategy(), 1..80)) {
        let mut nb = Notebook::new("prop");
        // Ids ever created — removed ones stay to exercise error paths.
        let mut ids: Vec<NoteId> = Vec::new();

        for (step, op) in ops.into_iter().enumerate() {
            match op {
                Op::Create => ids.push(nb.create_note("note")),
                Op::Link(a, b) => {
                    if let (Some(from), Some(to)) = (pick(&ids, a), pick(&ids, b)) {
                        let _ = nb.link_notes(from, to);
                    }
                }
                Op::Unlink(a, b) => {
                    if let (Some(from), Some(to)) = (pick(&ids, a), pick(&ids, b)) {
                        let _ = nb.unlink_notes(from, to);
                    }
                }
                Op::Remove(a) => {
                    if let Some(id) = pick(&ids, a) {
                        nb.remove_note(&id);
                    }
                }
            }
            prop_assert!(
                structural_violation(&nb, &ids).is_none(),
                "invariant failed immediately after step {step}: {:?}",
                structural_violation(&nb, &ids)
            );
        }
    }

    /// Serialization round-trips the complete semantic notebook state after
    /// both sides rebuild the derived backlink index.
    #[test]
    fn serde_roundtrip(ops in prop::collection::vec(op_strategy(), 1..40)) {
        let mut nb = Notebook::new("prop");
        let mut ids: Vec<NoteId> = Vec::new();
        for op in ops {
            match op {
                Op::Create => ids.push(nb.create_note("note")),
                Op::Link(a, b) => {
                    if let (Some(from), Some(to)) = (pick(&ids, a), pick(&ids, b)) {
                        let _ = nb.link_notes(from, to);
                    }
                }
                Op::Unlink(a, b) => {
                    if let (Some(from), Some(to)) = (pick(&ids, a), pick(&ids, b)) {
                        let _ = nb.unlink_notes(from, to);
                    }
                }
                Op::Remove(a) => {
                    if let Some(id) = pick(&ids, a) {
                        nb.remove_note(&id);
                    }
                }
            }
        }

        let json = serde_json::to_string(&nb).unwrap();
        let mut restored: Notebook = serde_json::from_str(&json).unwrap();
        let mut expected = nb.clone();
        expected.rebuild_backlinks();
        restored.rebuild_backlinks();

        prop_assert_eq!(persistent_value(&expected), persistent_value(&restored));
        prop_assert!(
            structural_violation(&restored, &ids).is_none(),
            "restored notebook invariant failed: {:?}",
            structural_violation(&restored, &ids)
        );
        for id in &ids {
            let expected_backlinks: HashSet<NoteId> =
                expected.get_backlinks(id).into_iter().collect();
            let restored_backlinks: HashSet<NoteId> =
                restored.get_backlinks(id).into_iter().collect();
            prop_assert_eq!(expected_backlinks, restored_backlinks);
        }
    }
}
