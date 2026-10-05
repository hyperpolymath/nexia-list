// SPDX-License-Identifier: MPL-2.0
//! Notebook - collection of notes with relationship tracking

use crate::agent::{note_matches, Agent};
use crate::note::{Note, NoteId};
use crate::wikilink::wikilink_targets;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during notebook operations
#[derive(Debug, Error)]
pub enum NotebookError {
    #[error("Note not found: {0}")]
    NoteNotFound(NoteId),

    #[error("Cannot create circular link")]
    CircularLink,
}

/// On-disk schema version written by this build.
///
/// * v1 — the original format (no `schema_version` key; files written before
///   versioning are read as v1).
/// * v2 — adds per-note `computed` fields (λδ formula sources).
///
/// v1 → v2 is purely additive, so migration is the serde default.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Version assumed for files that predate the `schema_version` key.
const LEGACY_SCHEMA_VERSION: u32 = 1;

/// The serde default for a missing `schema_version` key.
fn legacy_schema_version() -> u32 {
    LEGACY_SCHEMA_VERSION
}

/// Write `notes` in id order, so saving the same notebook twice produces the
/// same bytes (stable diffs, reproducible exports). Propagates serialiser errors.
fn serialize_sorted_notes<S: serde::Serializer>(
    notes: &HashMap<NoteId, Note>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let sorted: std::collections::BTreeMap<&NoteId, &Note> = notes.iter().collect();
    sorted.serialize(serializer)
}

/// Write the backlink index with keys and each source list in id order.
/// Propagates serialiser errors.
fn serialize_sorted_backlinks<S: serde::Serializer>(
    backlinks: &HashMap<NoteId, HashSet<NoteId>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let sorted: std::collections::BTreeMap<&NoteId, Vec<&NoteId>> = backlinks
        .iter()
        .map(|(target, sources)| {
            let mut sources: Vec<&NoteId> = sources.iter().collect();
            sources.sort();
            (target, sources)
        })
        .collect();
    sorted.serialize(serializer)
}

/// Lowercased title and content of every note, for case-insensitive search.
/// Rebuilt when the notebook's revision moves on (see [`Notebook::search`]).
#[derive(Debug, Clone, Default)]
struct SearchCache {
    built: bool,
    revision: u64,
    entries: Vec<(NoteId, String, String)>,
}

/// Why a stored notebook could not be loaded at all.
///
/// Recoverable damage (dangling links and the like) is *repaired* and
/// reported in a [`LoadReport`] instead; these are the unrecoverable cases.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Not JSON, truncated, or the wrong shape for a notebook.
    #[error("corrupt notebook data: {0}")]
    Corrupt(#[from] serde_json::Error),

    /// Written by a newer build. Refused rather than loaded lossily, since
    /// re-saving would silently drop whatever the newer format added.
    #[error("notebook schema v{found} is newer than this build supports (v{supported})")]
    UnsupportedSchema { found: u32, supported: u32 },
}

/// What [`Notebook::load_json`] had to change to make a stored notebook
/// consistent. Empty (`is_clean()`) for any file this build wrote itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadReport {
    /// Schema version found in the file.
    pub from_version: u32,
    /// Whether the file was migrated to [`CURRENT_SCHEMA_VERSION`].
    pub migrated: bool,
    /// Links whose target note does not exist, as `(source, target)`; removed.
    pub dangling_links: Vec<(NoteId, NoteId)>,
    /// Notes that linked to themselves; the self-link was removed.
    pub self_links: Vec<NoteId>,
    /// Repeated entries in a note's link list; collapsed to one.
    pub duplicate_links: usize,
    /// Notes stored under a map key different from their own `id`; re-keyed.
    pub rekeyed_notes: Vec<NoteId>,
    /// Notes dropped because another note already claimed the same `id`.
    pub dropped_duplicate_ids: Vec<NoteId>,
    /// Notes whose stored position or size was not a finite number (written
    /// as `null` by older builds); the geometry was cleared.
    pub invalid_geometry: Vec<NoteId>,
}

impl LoadReport {
    /// True when loading changed nothing but (possibly) the schema version.
    pub fn is_clean(&self) -> bool {
        self.dangling_links.is_empty()
            && self.self_links.is_empty()
            && self.duplicate_links == 0
            && self.rekeyed_notes.is_empty()
            && self.dropped_duplicate_ids.is_empty()
            && self.invalid_geometry.is_empty()
    }
}

/// A notebook containing a collection of interconnected notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    /// On-disk format version; see [`CURRENT_SCHEMA_VERSION`].
    #[serde(default = "legacy_schema_version")]
    schema_version: u32,

    /// All notes indexed by ID
    #[serde(serialize_with = "serialize_sorted_notes")]
    notes: HashMap<NoteId, Note>,

    /// Reverse index: for each note, which notes link TO it
    #[serde(default, serialize_with = "serialize_sorted_backlinks")]
    backlinks: HashMap<NoteId, HashSet<NoteId>>,

    /// Persistent saved queries (agents).
    #[serde(default)]
    agents: Vec<Agent>,

    /// In-memory mutation counter, bumped by every change (`touch`); keys
    /// derived caches. Not persisted. A counter rather than `modified_at`,
    /// whose clock has millisecond resolution on WebAssembly.
    #[serde(skip)]
    revision: u64,

    /// Lowercased note text for search; valid while its revision matches.
    #[serde(skip)]
    search_cache: std::cell::RefCell<SearchCache>,

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
            schema_version: CURRENT_SCHEMA_VERSION,
            notes: HashMap::new(),
            backlinks: HashMap::new(),
            agents: Vec::new(),
            revision: 0,
            search_cache: Default::default(),
            name: name.into(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Load a notebook from its JSON form, migrating older schemas and
    /// repairing recoverable damage. Returns the notebook and a report of
    /// migration and repairs; the backlink index is rebuilt from the notes.
    /// A missing schema version is treated as v1.
    ///
    /// # Errors
    /// Returns [`LoadError::Corrupt`] for invalid JSON or notebook data,
    /// including malformed stored backlinks, and [`LoadError::UnsupportedSchema`]
    /// for a successfully parsed schema newer than this build supports.
    pub fn load_json(json: &str) -> Result<(Notebook, LoadReport), LoadError> {
        let mut nb: Notebook = serde_json::from_str(json)?;
        if nb.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(LoadError::UnsupportedSchema {
                found: nb.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut report = LoadReport {
            from_version: nb.schema_version,
            migrated: nb.schema_version < CURRENT_SCHEMA_VERSION,
            ..LoadReport::default()
        };
        nb.schema_version = CURRENT_SCHEMA_VERSION;
        nb.repair(&mut report);
        Ok((nb, report))
    }

    /// The schema version this notebook will be written as.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Restore the structural invariants a hand-edited or damaged file may
    /// violate: map key == note id, ids unique, links point at existing notes,
    /// no self-links, no duplicate links. Clears non-finite geometry and
    /// records note and link repairs in `report`; rebuilding backlinks is
    /// not reported. For duplicate ids, keeps the note with the lowest map key.
    fn repair(&mut self, report: &mut LoadReport) {
        // Re-key under each note's own id. Iterate in key order so the
        // survivor of an id collision is deterministic.
        let mut entries: Vec<(NoteId, Note)> = self.notes.drain().collect();
        entries.sort_by_key(|(key, _)| *key);
        for (key, note) in entries {
            let id = note.id;
            if self.notes.contains_key(&id) {
                report.dropped_duplicate_ids.push(id);
                continue;
            }
            if key != id {
                report.rekeyed_notes.push(id);
            }
            self.notes.insert(id, note);
        }

        let existing: HashSet<NoteId> = self.notes.keys().copied().collect();
        let mut ids: Vec<NoteId> = existing.iter().copied().collect();
        ids.sort();
        for id in ids {
            let Some(note) = self.notes.get_mut(&id) else {
                continue;
            };
            if note.clear_non_finite_geometry() {
                report.invalid_geometry.push(id);
            }
            let mut seen = HashSet::new();
            let mut kept = Vec::with_capacity(note.links.len());
            for target in note.links.drain(..) {
                if target == id {
                    if !report.self_links.contains(&id) {
                        report.self_links.push(id);
                    }
                } else if !existing.contains(&target) {
                    report.dangling_links.push((id, target));
                } else if !seen.insert(target) {
                    report.duplicate_links += 1;
                } else {
                    kept.push(target);
                }
            }
            note.links = kept;
        }
        self.rebuild_backlinks();
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
            self.backlinks.entry(*target_id).or_default().insert(id);
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
        // A self-link would corrupt the backlink index: Note::add_link
        // refuses it silently, so the backlink below must not be recorded.
        if from == to {
            return Err(NotebookError::CircularLink);
        }
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

    /// Set a note's content and add links for any `[[Title]]` it now names
    /// (resolved case-insensitively against existing titles). Additive: a
    /// wiki-link is never *removed* when its text is deleted — use the
    /// explicit unlink for that. Returns the ids of notes newly linked to.
    pub fn set_content(&mut self, id: &NoteId, content: impl Into<String>) -> Vec<NoteId> {
        let content = content.into();
        let targets = wikilink_targets(&content);
        match self.notes.get_mut(id) {
            Some(note) => {
                note.content = content;
                note.touch();
            }
            None => return Vec::new(),
        }
        // Resolve titles → ids once.
        let title_map: HashMap<String, NoteId> = self
            .notes
            .values()
            .map(|n| (n.title.to_lowercase(), n.id))
            .collect();

        let mut newly_linked = Vec::new();
        for target in targets {
            if let Some(&target_id) = title_map.get(&target.to_lowercase()) {
                let already = self.get_note(id).is_some_and(|n| n.links_to(&target_id));
                if target_id != *id && !already && self.link_notes(*id, target_id).is_ok() {
                    newly_linked.push(target_id);
                }
            }
        }
        self.touch();
        newly_linked
    }

    /// Define computed field `name` on note `id` as the λδ `formula`, without
    /// validating or evaluating it. Updates the note and notebook timestamps.
    /// Returns [`NotebookError::NoteNotFound`] if the note does not exist.
    pub fn set_computed(
        &mut self,
        id: &NoteId,
        name: impl Into<String>,
        formula: impl Into<String>,
    ) -> Result<(), NotebookError> {
        let note = self
            .notes
            .get_mut(id)
            .ok_or(NotebookError::NoteNotFound(*id))?;
        note.set_computed(name, formula);
        self.touch();
        Ok(())
    }

    /// Remove computed field `name` from note `id`. Returns whether it existed,
    /// updating the note and notebook timestamps only when it did.
    /// Returns [`NotebookError::NoteNotFound`] if the note does not exist.
    pub fn remove_computed(&mut self, id: &NoteId, name: &str) -> Result<bool, NotebookError> {
        let note = self
            .notes
            .get_mut(id)
            .ok_or(NotebookError::NoteNotFound(*id))?;
        let existed = note.remove_computed(name);
        if existed {
            self.touch();
        }
        Ok(existed)
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

    /// Search notes by title or content (case-insensitive substring match).
    ///
    /// An empty query matches every note; results have no guaranteed order.
    /// Lowercased note text is cached until the notebook revision changes.
    pub fn search(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        let mut cache = self.search_cache.borrow_mut();
        if !cache.built || cache.revision != self.revision {
            cache.entries = self
                .notes
                .values()
                .map(|n| (n.id, n.title.to_lowercase(), n.content.to_lowercase()))
                .collect();
            cache.revision = self.revision;
            cache.built = true;
        }
        cache
            .entries
            .iter()
            .filter(|(_, title, content)| {
                title.contains(&query_lower) || content.contains(&query_lower)
            })
            .filter_map(|(id, _, _)| self.notes.get(id))
            .collect()
    }

    /// Rebuild the backlinks index from the notes' outgoing links.
    ///
    /// The index is persisted alongside the notes, but a hand-edited or
    /// older file may carry a stale one — loading always rebuilds instead
    /// of trusting it.
    pub fn rebuild_backlinks(&mut self) {
        self.backlinks.clear();
        for (id, note) in &self.notes {
            for target_id in &note.links {
                self.backlinks.entry(*target_id).or_default().insert(*id);
            }
        }
    }

    // ── Agents (persistent saved queries) ────────────────────────────

    /// All agents, in insertion order.
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// Create and store an agent; returns its id.
    pub fn add_agent(&mut self, name: impl Into<String>, query: impl Into<String>) -> NoteId {
        let agent = Agent::new(name, query);
        let id = agent.id;
        self.agents.push(agent);
        self.touch();
        id
    }

    /// Remove an agent by id; returns whether one was removed.
    pub fn remove_agent(&mut self, id: &Uuid) -> bool {
        let before = self.agents.len();
        self.agents.retain(|a| &a.id != id);
        let removed = self.agents.len() != before;
        if removed {
            self.touch();
        }
        removed
    }

    /// The note ids matching a raw query string.
    pub fn run_query(&self, query: &str) -> Vec<NoteId> {
        self.notes
            .values()
            .filter(|note| note_matches(note, query))
            .map(|note| note.id)
            .collect()
    }

    /// The note ids collected by a stored agent (empty if the id is unknown).
    pub fn run_agent(&self, id: &Uuid) -> Vec<NoteId> {
        match self.agents.iter().find(|a| &a.id == id) {
            Some(agent) => self.run_query(&agent.query),
            None => Vec::new(),
        }
    }

    /// Record a change: new modified time and revision.
    fn touch(&mut self) {
        self.modified_at = chrono::Utc::now();
        self.revision = self.revision.wrapping_add(1);
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
    fn search_cache_follows_every_kind_of_change() {
        let mut nb = Notebook::new("s");
        let a = nb.create_note("Alpha");
        assert_eq!(nb.search("alp").len(), 1);
        // Renaming through get_note_mut.
        nb.get_note_mut(&a).unwrap().title = "Gamma".into();
        assert!(nb.search("alp").is_empty());
        assert_eq!(nb.search("GAM").len(), 1);
        // Content through set_content.
        nb.set_content(&a, "Kelvin \u{212A} and Straße");
        assert_eq!(nb.search("straße").len(), 1);
        assert_eq!(nb.search("k").len(), 1);
        // Adding and removing notes.
        let b = nb.create_note("Beta");
        assert_eq!(nb.search("beta").len(), 1);
        nb.remove_note(&b);
        assert!(nb.search("beta").is_empty());
    }

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
    fn test_set_content_derives_wikilinks() {
        let mut notebook = Notebook::new("Test");
        let a = notebook.create_note("Alpha");
        let b = notebook.create_note("Beta");

        // Case-insensitive resolution; self-reference ignored.
        let linked = notebook.set_content(&a, "see [[beta]] and [[Alpha]]");
        assert_eq!(linked, vec![b]);
        assert!(notebook.get_note(&a).unwrap().links_to(&b));
        assert!(!notebook.get_note(&a).unwrap().links_to(&a));
        assert_eq!(notebook.get_backlinks(&b), vec![a]);

        // Unresolved titles are ignored; already-linked targets aren't re-added.
        let linked2 = notebook.set_content(&a, "[[beta]] again and [[Ghost]]");
        assert!(linked2.is_empty());
        assert_eq!(notebook.get_note(&a).unwrap().links.len(), 1);
    }

    #[test]
    fn test_agents_collect_run_and_persist() {
        let mut notebook = Notebook::new("Test");
        let todo = notebook.create_note("Buy milk");
        if let Some(n) = notebook.get_note_mut(&todo) {
            n.set_attribute("status", serde_json::json!("todo"));
        }
        let done = notebook.create_note("Ship release");
        if let Some(n) = notebook.get_note_mut(&done) {
            n.set_attribute("status", serde_json::json!("done"));
        }

        let agent = notebook.add_agent("Open tasks", "attr:status=todo");
        assert_eq!(notebook.agents().len(), 1);
        assert_eq!(notebook.run_agent(&agent), vec![todo]);

        // Survives a serde round-trip.
        let json = serde_json::to_string(&notebook).unwrap();
        let restored: Notebook = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agents().len(), 1);
        assert_eq!(restored.run_agent(&agent), vec![todo]);

        assert!(notebook.remove_agent(&agent));
        assert!(notebook.agents().is_empty());
        assert!(notebook.run_agent(&agent).is_empty());
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
    fn test_self_link_rejected() {
        let mut notebook = Notebook::new("Test");
        let id = notebook.create_note("Note");

        assert!(matches!(
            notebook.link_notes(id, id),
            Err(NotebookError::CircularLink)
        ));
        // The backlink index must stay untouched.
        assert!(notebook.get_backlinks(&id).is_empty());
    }

    #[test]
    fn test_rebuild_backlinks() {
        let mut notebook = Notebook::new("Test");
        let id1 = notebook.create_note("Note 1");
        let id2 = notebook.create_note("Note 2");
        notebook.link_notes(id1, id2).unwrap();

        // Corrupt the index, then rebuild
        notebook.backlinks.clear();
        assert!(notebook.get_backlinks(&id2).is_empty());

        notebook.rebuild_backlinks();
        assert_eq!(notebook.get_backlinks(&id2), vec![id1]);
        assert!(notebook.get_backlinks(&id1).is_empty());
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
