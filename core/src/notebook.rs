// SPDX-License-Identifier: AGPL-3.0-or-later
//! Notebook - collection of notes with relationship tracking

use crate::note::{Note, NoteId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during notebook operations
#[derive(Debug, Error)]
pub enum NotebookError {
    #[error("Note not found: {0}")]
    NoteNotFound(NoteId),

    #[error("Cannot create circular link")]
    CircularLink,
}

/// A notebook containing a collection of interconnected notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    /// All notes indexed by ID
    notes: HashMap<NoteId, Note>,

    /// Reverse index: for each note, which notes link TO it
    #[serde(default)]
    backlinks: HashMap<NoteId, HashSet<NoteId>>,

    /// Notebook metadata
    pub name: String,

    /// When the notebook was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the notebook was last modified
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

impl Notebook {
    /// Create a new empty notebook
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            notes: HashMap::new(),
            backlinks: HashMap::new(),
            name: name.into(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Get the number of notes
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// Check if the notebook is empty
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Add a note to the notebook
    pub fn add_note(&mut self, note: Note) -> NoteId {
        let id = note.id;

        // Update backlinks for any links this note has
        for target_id in &note.links {
            self.backlinks
                .entry(*target_id)
                .or_default()
                .insert(id);
        }

        self.notes.insert(id, note);
        self.touch();
        id
    }

    /// Create a new note with the given title and add it
    pub fn create_note(&mut self, title: impl Into<String>) -> NoteId {
        let note = Note::new(title);
        self.add_note(note)
    }

    /// Get a note by ID
    pub fn get_note(&self, id: &NoteId) -> Option<&Note> {
        self.notes.get(id)
    }

    /// Get a mutable reference to a note
    pub fn get_note_mut(&mut self, id: &NoteId) -> Option<&mut Note> {
        self.touch();
        self.notes.get_mut(id)
    }

    /// Remove a note and all links to/from it
    pub fn remove_note(&mut self, id: &NoteId) -> Option<Note> {
        if let Some(note) = self.notes.remove(id) {
            // Remove this note from backlinks of notes it linked to
            for target_id in &note.links {
                if let Some(backlink_set) = self.backlinks.get_mut(target_id) {
                    backlink_set.remove(id);
                }
            }

            // Remove links from other notes that pointed to this one
            if let Some(sources) = self.backlinks.remove(id) {
                for source_id in sources {
                    if let Some(source_note) = self.notes.get_mut(&source_id) {
                        source_note.remove_link(id);
                    }
                }
            }

            self.touch();
            Some(note)
        } else {
            None
        }
    }

    /// Create a link between two notes
    pub fn link_notes(&mut self, from: NoteId, to: NoteId) -> Result<(), NotebookError> {
        // Verify both notes exist
        if !self.notes.contains_key(&from) {
            return Err(NotebookError::NoteNotFound(from));
        }
        if !self.notes.contains_key(&to) {
            return Err(NotebookError::NoteNotFound(to));
        }

        // Add the link
        if let Some(note) = self.notes.get_mut(&from) {
            note.add_link(to);
        }

        // Update backlinks
        self.backlinks.entry(to).or_default().insert(from);
        self.touch();

        Ok(())
    }

    /// Remove a link between two notes
    pub fn unlink_notes(&mut self, from: NoteId, to: NoteId) -> Result<(), NotebookError> {
        if let Some(note) = self.notes.get_mut(&from) {
            note.remove_link(&to);
        } else {
            return Err(NotebookError::NoteNotFound(from));
        }

        if let Some(backlink_set) = self.backlinks.get_mut(&to) {
            backlink_set.remove(&from);
        }

        self.touch();
        Ok(())
    }

    /// Get all notes that link TO the given note
    pub fn get_backlinks(&self, id: &NoteId) -> Vec<NoteId> {
        self.backlinks
            .get(id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all notes
    pub fn all_notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }

    /// Get all note IDs
    pub fn all_note_ids(&self) -> impl Iterator<Item = &NoteId> {
        self.notes.keys()
    }

    /// Search notes by title (case-insensitive substring match)
    pub fn search_by_title(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        self.notes
            .values()
            .filter(|note| note.title.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Search notes by content (case-insensitive substring match)
    pub fn search_by_content(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        self.notes
            .values()
            .filter(|note| note.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Search notes by title or content
    pub fn search(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        self.notes
            .values()
            .filter(|note| {
                note.title.to_lowercase().contains(&query_lower)
                    || note.content.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Update the modified timestamp
    fn touch(&mut self) {
        self.modified_at = chrono::Utc::now();
    }
}

impl Default for Notebook {
    fn default() -> Self {
        Self::new("Untitled Notebook")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_notebook() {
        let notebook = Notebook::new("Test");
        assert_eq!(notebook.name, "Test");
        assert!(notebook.is_empty());
    }

    #[test]
    fn test_add_and_get_note() {
        let mut notebook = Notebook::new("Test");
        let id = notebook.create_note("First Note");

        let note = notebook.get_note(&id).unwrap();
        assert_eq!(note.title, "First Note");
        assert_eq!(notebook.len(), 1);
    }

    #[test]
    fn test_link_notes() {
        let mut notebook = Notebook::new("Test");
        let id1 = notebook.create_note("Note 1");
        let id2 = notebook.create_note("Note 2");

        notebook.link_notes(id1, id2).unwrap();

        let note1 = notebook.get_note(&id1).unwrap();
        assert!(note1.links_to(&id2));

        let backlinks = notebook.get_backlinks(&id2);
        assert!(backlinks.contains(&id1));
    }

    #[test]
    fn test_remove_note_cleans_links() {
        let mut notebook = Notebook::new("Test");
        let id1 = notebook.create_note("Note 1");
        let id2 = notebook.create_note("Note 2");
        let id3 = notebook.create_note("Note 3");

        // id1 -> id2 -> id3
        notebook.link_notes(id1, id2).unwrap();
        notebook.link_notes(id2, id3).unwrap();

        // Remove id2
        notebook.remove_note(&id2);

        // id1 should no longer have the link
        let note1 = notebook.get_note(&id1).unwrap();
        assert!(!note1.links_to(&id2));

        // id3 should have no backlinks
        assert!(notebook.get_backlinks(&id3).is_empty());
    }

    #[test]
    fn test_search() {
        let mut notebook = Notebook::new("Test");

        let id1 = notebook.create_note("Meeting Notes");
        if let Some(note) = notebook.get_note_mut(&id1) {
            note.content = "Discussion about project timeline".into();
        }

        let id2 = notebook.create_note("Project Plan");
        if let Some(note) = notebook.get_note_mut(&id2) {
            note.content = "Milestones and deliverables".into();
        }

        // Search by title
        let results = notebook.search_by_title("meeting");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);

        // Search by content
        let results = notebook.search_by_content("project");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);

        // Combined search
        let results = notebook.search("project");
        assert_eq!(results.len(), 2); // Both match
    }
}
