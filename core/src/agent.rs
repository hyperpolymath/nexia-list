// SPDX-License-Identifier: MPL-2.0
//! Agents — persistent saved queries, the feature Nexia-List is named for.
//!
//! An agent stores a small query written in a whitespace-separated DSL; the
//! terms are ANDed together. Supported terms:
//!
//! - `word` — bare text, matched against title + content
//! - `title:word` — text matched against the title only
//! - `attr:key=value` — an attribute equal to a string/number value
//! - `linksto:<uuid>` — the note links to that id
//!
//! Quoting is not supported; multi-word phrases use several bare terms.

use crate::note::{Note, NoteId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persistent saved query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub query: String,
}

impl Agent {
    pub fn new(name: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            query: query.into(),
        }
    }
}

/// A single parsed query term.
#[derive(Debug, Clone, PartialEq)]
enum Term {
    Text(String),
    Title(String),
    Attr(String, String),
    LinksTo(NoteId),
    /// A term that can never match (e.g. `linksto:` with an invalid uuid).
    Never,
}

/// Parse a raw query string into ANDed terms. Total: never panics.
fn parse(query: &str) -> Vec<Term> {
    query
        .split_whitespace()
        .filter_map(|tok| {
            if let Some(rest) = tok.strip_prefix("title:") {
                non_empty(rest).map(|s| Term::Title(s.to_lowercase()))
            } else if let Some(rest) = tok.strip_prefix("attr:") {
                match rest.split_once('=') {
                    Some((k, v)) if !k.is_empty() => Some(Term::Attr(k.to_string(), v.to_string())),
                    _ => Some(Term::Never),
                }
            } else if let Some(rest) = tok.strip_prefix("linksto:") {
                Some(
                    Uuid::parse_str(rest)
                        .map(Term::LinksTo)
                        .unwrap_or(Term::Never),
                )
            } else {
                non_empty(tok).map(|s| Term::Text(s.to_lowercase()))
            }
        })
        .collect()
}

fn non_empty(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Does a note satisfy every term of the query? An empty query matches nothing
/// (so a blank agent does not "collect everything").
pub fn note_matches(note: &Note, query: &str) -> bool {
    let terms = parse(query);
    if terms.is_empty() {
        return false;
    }
    terms.iter().all(|term| match term {
        Term::Text(t) => {
            note.title.to_lowercase().contains(t) || note.content.to_lowercase().contains(t)
        }
        Term::Title(t) => note.title.to_lowercase().contains(t),
        Term::Attr(key, value) => match note.get_attribute(key) {
            Some(v) => attribute_equals(v, value),
            None => false,
        },
        Term::LinksTo(id) => note.links_to(id),
        Term::Never => false,
    })
}

/// Compare a JSON attribute value against the query's string form. Strings
/// compare by content; numbers/bools compare by their textual rendering so
/// `attr:done=true` and `attr:count=3` work without quoting.
fn attribute_equals(value: &serde_json::Value, expected: &str) -> bool {
    match value {
        serde_json::Value::String(s) => s == expected,
        serde_json::Value::Bool(b) => *b == (expected == "true"),
        serde_json::Value::Null => expected == "null",
        // Numbers: compare their canonical rendering (e.g. `3`, `1.5`).
        serde_json::Value::Number(n) => {
            let rendered = n.to_string();
            rendered == expected
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn note(title: &str, content: &str) -> Note {
        let mut n = Note::new(title);
        n.content = content.to_string();
        n
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert!(!note_matches(&note("A", "B"), "   "));
    }

    #[test]
    fn text_and_title_terms() {
        let n = note("Meeting Notes", "about the roadmap");
        assert!(note_matches(&n, "roadmap"));
        assert!(note_matches(&n, "MEETING roadmap")); // ANDed, case-insensitive
        assert!(note_matches(&n, "title:meeting"));
        assert!(!note_matches(&n, "title:roadmap")); // only in content
        assert!(!note_matches(&n, "roadmap missing"));
    }

    #[test]
    fn attribute_terms() {
        let mut n = note("Task", "");
        n.set_attribute("status", json!("todo"));
        n.set_attribute("count", json!(3));
        n.set_attribute("done", json!(true));
        assert!(note_matches(&n, "attr:status=todo"));
        assert!(!note_matches(&n, "attr:status=done"));
        assert!(note_matches(&n, "attr:count=3"));
        assert!(note_matches(&n, "attr:done=true"));
        assert!(!note_matches(&n, "attr:missing=x"));
    }

    #[test]
    fn linksto_term() {
        let target = Uuid::new_v4();
        let mut n = note("Source", "");
        n.add_link(target);
        assert!(note_matches(&n, &format!("linksto:{target}")));
        assert!(!note_matches(&n, &format!("linksto:{}", Uuid::new_v4())));
        assert!(!note_matches(&n, "linksto:not-a-uuid")); // Never
    }
}
