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

proptest! {
    /// After any sequence of operations, the backlinks index is exactly the
    /// inverse of the union of the notes' outgoing links.
    #[test]
    fn backlinks_are_exact_inverse(ops in prop::collection::vec(op_strategy(), 1..80)) {
        let mut nb = Notebook::new("prop");
        // Ids ever created — removed ones stay to exercise error paths.
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

        for id in nb.all_note_ids() {
            let expected: HashSet<NoteId> =
                nb.all_notes().filter(|n| n.links_to(id)).map(|n| n.id).collect();
            let actual: HashSet<NoteId> = nb.get_backlinks(id).into_iter().collect();
            prop_assert_eq!(expected, actual, "backlink index diverged for {}", id);
        }

        // No dangling targets: every link points at a live note.
        for note in nb.all_notes() {
            for target in &note.links {
                prop_assert!(nb.get_note(target).is_some(), "dangling link to {}", target);
            }
        }
    }

    /// Serialization round-trips without losing notes or links.
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
                _ => {}
            }
        }

        let json = serde_json::to_string(&nb).unwrap();
        let nb2: Notebook = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(nb.len(), nb2.len());
        for note in nb.all_notes() {
            let restored = nb2.get_note(&note.id).unwrap();
            prop_assert_eq!(&note.links, &restored.links);
            prop_assert_eq!(&note.title, &restored.title);
        }
    }
}
