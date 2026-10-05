// SPDX-License-Identifier: MPL-2.0
//! Note data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// Unique identifier for a note
pub type NoteId = Uuid;

/// 2D position on the spatial canvas
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Whether both coordinates are finite (storable as JSON numbers).
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Default for Point2D {
    fn default() -> Self {
        Self::origin()
    }
}

/// True when `position` should not be written: absent, or not representable
/// in JSON.
fn point_absent_or_non_finite(position: &Option<Point2D>) -> bool {
    position.is_none_or(|p| !p.is_finite())
}

/// True when `size` should not be written: absent, or not representable in
/// JSON.
fn size_absent_or_non_finite(size: &Option<(f64, f64)>) -> bool {
    size.is_none_or(|(w, h)| !(w.is_finite() && h.is_finite()))
}

/// Read a position whose coordinates may be `null` (as written by builds that
/// serialized NaN); a `null` becomes NaN for the load-time repair to clear.
fn deserialize_lenient_point<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Point2D>, D::Error> {
    #[derive(Deserialize)]
    struct Lenient {
        x: Option<f64>,
        y: Option<f64>,
    }
    let raw: Option<Lenient> = Option::deserialize(deserializer)?;
    Ok(raw.map(|p| Point2D::new(p.x.unwrap_or(f64::NAN), p.y.unwrap_or(f64::NAN))))
}

/// Read a size whose components may be `null`; see [`deserialize_lenient_point`].
fn deserialize_lenient_size<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<(f64, f64)>, D::Error> {
    let raw: Option<(Option<f64>, Option<f64>)> = Option::deserialize(deserializer)?;
    Ok(raw.map(|(w, h)| (w.unwrap_or(f64::NAN), h.unwrap_or(f64::NAN))))
}

/// Write attributes in key order, so a saved note is byte-stable.
fn serialize_sorted_attributes<S: serde::Serializer>(
    attributes: &HashMap<String, serde_json::Value>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let sorted: BTreeMap<&String, &serde_json::Value> = attributes.iter().collect();
    sorted.serialize(serializer)
}

/// A single note in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Unique identifier
    pub id: NoteId,

    /// Note title
    pub title: String,

    /// Note content (plain text for MVP, rich text later)
    pub content: String,

    /// Position on the spatial canvas (None if not placed). A non-finite
    /// position is never written (JSON has no NaN/Infinity, and serde_json
    /// would emit `null`, making the whole file unloadable); a `null`
    /// coordinate in an existing file is read leniently so loading can repair
    /// it (see [`Note::clear_non_finite_geometry`]).
    #[serde(
        default,
        skip_serializing_if = "point_absent_or_non_finite",
        deserialize_with = "deserialize_lenient_point"
    )]
    pub position: Option<Point2D>,

    /// Size on canvas (width, height); same non-finite handling as `position`.
    #[serde(
        default,
        skip_serializing_if = "size_absent_or_non_finite",
        deserialize_with = "deserialize_lenient_size"
    )]
    pub size: Option<(f64, f64)>,

    /// When the note was created
    pub created_at: DateTime<Utc>,

    /// When the note was last modified
    pub modified_at: DateTime<Utc>,

    /// Outgoing links to other notes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<NoteId>,

    /// Prototype note for inheritance (None if no prototype)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prototype: Option<NoteId>,

    /// Custom attributes
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        serialize_with = "serialize_sorted_attributes"
    )]
    pub attributes: HashMap<String, serde_json::Value>,

    /// Computed fields: field name → λδ formula source. The *source* is
    /// persisted; values are derived on demand (see
    /// [`crate::lambdadelta_host::eval_computed_fields`]) and never stored, so
    /// a reload can never show a stale value. Ordered for stable output.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub computed: BTreeMap<String, String>,
}

impl Note {
    /// Create a new note with default values
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            content: String::new(),
            position: None,
            size: None,
            created_at: now,
            modified_at: now,
            links: Vec::new(),
            prototype: None,
            attributes: HashMap::new(),
            computed: BTreeMap::new(),
        }
    }

    /// Create a note with a specific position on the canvas
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Some(Point2D::new(x, y));
        self
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified_at = Utc::now();
    }

    /// Add a link to another note
    pub fn add_link(&mut self, target: NoteId) {
        if !self.links.contains(&target) && target != self.id {
            self.links.push(target);
            self.touch();
        }
    }

    /// Remove a link to another note
    pub fn remove_link(&mut self, target: &NoteId) -> bool {
        if let Some(pos) = self.links.iter().position(|id| id == target) {
            self.links.remove(pos);
            self.touch();
            true
        } else {
            false
        }
    }

    /// Check if this note links to another
    pub fn links_to(&self, target: &NoteId) -> bool {
        self.links.contains(target)
    }

    /// Set an attribute value
    pub fn set_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
        self.touch();
    }

    /// Get an attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }

    /// Drop a non-finite position or size (the note becomes unplaced /
    /// default-sized). Returns whether anything was cleared.
    pub fn clear_non_finite_geometry(&mut self) -> bool {
        let mut cleared = false;
        if self.position.is_some_and(|p| !p.is_finite()) {
            self.position = None;
            cleared = true;
        }
        if self
            .size
            .is_some_and(|(w, h)| !(w.is_finite() && h.is_finite()))
        {
            self.size = None;
            cleared = true;
        }
        cleared
    }

    /// Define (or redefine) the computed field `name` as the λδ `formula`.
    pub fn set_computed(&mut self, name: impl Into<String>, formula: impl Into<String>) {
        self.computed.insert(name.into(), formula.into());
        self.touch();
    }

    /// Remove the computed field `name`. Returns whether it existed.
    pub fn remove_computed(&mut self, name: &str) -> bool {
        let existed = self.computed.remove(name).is_some();
        if existed {
            self.touch();
        }
        existed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_note() {
        let note = Note::new("Test Note");
        assert_eq!(note.title, "Test Note");
        assert!(note.content.is_empty());
        assert!(note.position.is_none());
        assert!(note.links.is_empty());
    }

    #[test]
    fn test_note_with_position() {
        let note = Note::new("Positioned").with_position(100.0, 200.0);
        assert_eq!(note.position, Some(Point2D::new(100.0, 200.0)));
    }

    #[test]
    fn test_add_link() {
        let mut note = Note::new("Source");
        let target_id = Uuid::new_v4();

        note.add_link(target_id);
        assert!(note.links_to(&target_id));

        // Adding same link twice should not duplicate
        note.add_link(target_id);
        assert_eq!(note.links.len(), 1);
    }

    #[test]
    fn test_remove_link() {
        let mut note = Note::new("Source");
        let target_id = Uuid::new_v4();

        note.add_link(target_id);
        assert!(note.remove_link(&target_id));
        assert!(!note.links_to(&target_id));

        // Removing non-existent link returns false
        assert!(!note.remove_link(&target_id));
    }

    #[test]
    fn test_self_link_prevented() {
        let mut note = Note::new("Self");
        let self_id = note.id;

        note.add_link(self_id);
        assert!(note.links.is_empty(), "Should not allow self-links");
    }
}
