// SPDX-License-Identifier: AGPL-3.0-or-later
//! Note data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl Default for Point2D {
    fn default() -> Self {
        Self::origin()
    }
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

    /// Position on the spatial canvas (None if not placed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Point2D>,

    /// Size on canvas (width, height)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
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
