// SPDX-License-Identifier: MPL-2.0
//! WASM bindings — the browser-facing API of the core.
//!
//! The UI keeps a read model and applies the values returned here; every
//! mutation returns only what changed (a single note view, or a delta for
//! operations that touch link topology), never the whole notebook.
//!
//! View types serialize with camelCase keys to match the UI's records; the
//! on-disk JSON format (snake_case, see `storage`) is unaffected.

use crate::note::{Note, Point2D};
use crate::notebook::Notebook;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    // json_compatible(): maps become plain JS objects, not Map instances.
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn err(msg: impl std::fmt::Display) -> JsValue {
    // A real JS Error (not a bare string) so callers can read .message.
    JsValue::from(JsError::new(&msg.to_string()))
}

fn parse_id(id: &str) -> Result<Uuid, JsValue> {
    Uuid::parse_str(id).map_err(|_| err(format!("Invalid note id: {id}")))
}

/// UI-facing shape of a note. Optional fields are omitted when absent
/// (ReScript reads absent as None); `links`/`attributes` are always present.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteView {
    id: String,
    title: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<Point2D>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<(f64, f64)>,
    created_at: String,
    modified_at: String,
    links: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prototype: Option<String>,
    attributes: HashMap<String, serde_json::Value>,
}

impl From<&Note> for NoteView {
    fn from(note: &Note) -> Self {
        Self {
            id: note.id.to_string(),
            title: note.title.clone(),
            content: note.content.clone(),
            position: note.position,
            size: note.size,
            created_at: note.created_at.to_rfc3339(),
            modified_at: note.modified_at.to_rfc3339(),
            links: note.links.iter().map(Uuid::to_string).collect(),
            prototype: note.prototype.map(|id| id.to_string()),
            attributes: note.attributes.clone(),
        }
    }
}

/// Result of a mutation that touches more than one note. The UI merges it:
/// upsert `changed`, drop `removed` (notes and their backlink entries),
/// replace each key present in `backlinks`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeltaView {
    changed: Vec<NoteView>,
    removed: Vec<String>,
    backlinks: HashMap<String, Vec<String>>,
}

/// Full notebook snapshot, used on init/new/load.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NotebookView {
    notes: HashMap<String, NoteView>,
    backlinks: HashMap<String, Vec<String>>,
    name: String,
    created_at: String,
    modified_at: String,
}

impl From<&Notebook> for NotebookView {
    fn from(nb: &Notebook) -> Self {
        let notes = nb
            .all_notes()
            .map(|note| (note.id.to_string(), NoteView::from(note)))
            .collect();
        let backlinks = nb
            .all_note_ids()
            .filter_map(|id| {
                let sources = nb.get_backlinks(id);
                if sources.is_empty() {
                    None
                } else {
                    Some((
                        id.to_string(),
                        sources.iter().map(Uuid::to_string).collect(),
                    ))
                }
            })
            .collect();
        Self {
            notes,
            backlinks,
            name: nb.name.clone(),
            created_at: nb.created_at.to_rfc3339(),
            modified_at: nb.modified_at.to_rfc3339(),
        }
    }
}

#[wasm_bindgen]
pub struct WasmNotebook {
    inner: Notebook,
}

#[wasm_bindgen]
impl WasmNotebook {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> WasmNotebook {
        WasmNotebook {
            inner: Notebook::new(name),
        }
    }

    /// Deserialize from the on-disk JSON format. Backlinks are rebuilt
    /// rather than trusted.
    pub fn from_json(json: &str) -> Result<WasmNotebook, JsValue> {
        let mut inner: Notebook =
            serde_json::from_str(json).map_err(|e| err(format!("Invalid notebook JSON: {e}")))?;
        inner.rebuild_backlinks();
        Ok(WasmNotebook { inner })
    }

    /// Serialize to the on-disk JSON format (pretty-printed, snake_case).
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(err)
    }

    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        to_js(&NotebookView::from(&self.inner))
    }

    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn create_note(&mut self, title: &str) -> Result<JsValue, JsValue> {
        let id = self.inner.create_note(title);
        self.note_view(&id.to_string())
    }

    pub fn create_note_at(&mut self, title: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let note = Note::new(title).with_position(x, y);
        let id = self.inner.add_note(note);
        self.note_view(&id.to_string())
    }

    pub fn get_note(&self, id: &str) -> Result<JsValue, JsValue> {
        let id = parse_id(id)?;
        match self.inner.get_note(&id) {
            Some(note) => to_js(&NoteView::from(note)),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    pub fn update_title(&mut self, id: &str, title: &str) -> Result<JsValue, JsValue> {
        self.with_note(id, |note| {
            note.title = title.to_string();
            note.touch();
        })
    }

    pub fn update_content(&mut self, id: &str, content: &str) -> Result<JsValue, JsValue> {
        self.with_note(id, |note| {
            note.content = content.to_string();
            note.touch();
        })
    }

    pub fn move_note(&mut self, id: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        self.with_note(id, |note| {
            note.position = Some(Point2D::new(x, y));
            note.touch();
        })
    }

    pub fn resize_note(&mut self, id: &str, width: f64, height: f64) -> Result<JsValue, JsValue> {
        self.with_note(id, |note| {
            note.size = Some((width, height));
            note.touch();
        })
    }

    pub fn set_attribute(&mut self, id: &str, key: &str, value: &str) -> Result<JsValue, JsValue> {
        let parsed: serde_json::Value =
            serde_json::from_str(value).map_err(|e| err(format!("Invalid attribute JSON: {e}")))?;
        self.with_note(id, |note| note.set_attribute(key, parsed))
    }

    /// Delete a note. The delta carries the notes that lost an outgoing
    /// link and the backlink entries of the deleted note's targets.
    pub fn delete_note(&mut self, id: &str) -> Result<JsValue, JsValue> {
        let uuid = parse_id(id)?;
        let sources = self.inner.get_backlinks(&uuid);
        let removed_note = self
            .inner
            .remove_note(&uuid)
            .ok_or_else(|| err(format!("Note not found: {id}")))?;

        let changed = sources
            .iter()
            .filter_map(|source_id| self.inner.get_note(source_id))
            .map(NoteView::from)
            .collect();
        let backlinks = removed_note
            .links
            .iter()
            .map(|target| {
                (
                    target.to_string(),
                    self.inner
                        .get_backlinks(target)
                        .iter()
                        .map(Uuid::to_string)
                        .collect(),
                )
            })
            .collect();

        to_js(&DeltaView {
            changed,
            removed: vec![uuid.to_string()],
            backlinks,
        })
    }

    pub fn link(&mut self, from: &str, to: &str) -> Result<JsValue, JsValue> {
        let from_id = parse_id(from)?;
        let to_id = parse_id(to)?;
        self.inner.link_notes(from_id, to_id).map_err(err)?;
        self.link_delta(from_id, to_id)
    }

    pub fn unlink(&mut self, from: &str, to: &str) -> Result<JsValue, JsValue> {
        let from_id = parse_id(from)?;
        let to_id = parse_id(to)?;
        self.inner.unlink_notes(from_id, to_id).map_err(err)?;
        self.link_delta(from_id, to_id)
    }

    pub fn backlinks(&self, id: &str) -> Result<Vec<String>, JsValue> {
        let id = parse_id(id)?;
        Ok(self
            .inner
            .get_backlinks(&id)
            .iter()
            .map(Uuid::to_string)
            .collect())
    }

    /// Case-insensitive substring search over titles and content.
    pub fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }
        self.inner
            .search(query)
            .iter()
            .map(|note| note.id.to_string())
            .collect()
    }
}

impl WasmNotebook {
    fn note_view(&self, id: &str) -> Result<JsValue, JsValue> {
        let uuid = parse_id(id)?;
        let note = self
            .inner
            .get_note(&uuid)
            .ok_or_else(|| err(format!("Note not found: {id}")))?;
        to_js(&NoteView::from(note))
    }

    fn with_note(&mut self, id: &str, mutate: impl FnOnce(&mut Note)) -> Result<JsValue, JsValue> {
        let uuid = parse_id(id)?;
        let note = self
            .inner
            .get_note_mut(&uuid)
            .ok_or_else(|| err(format!("Note not found: {id}")))?;
        mutate(note);
        self.note_view(id)
    }

    fn link_delta(&self, from: Uuid, to: Uuid) -> Result<JsValue, JsValue> {
        let changed = self
            .inner
            .get_note(&from)
            .map(NoteView::from)
            .into_iter()
            .collect();
        let mut backlinks = HashMap::new();
        backlinks.insert(
            to.to_string(),
            self.inner
                .get_backlinks(&to)
                .iter()
                .map(Uuid::to_string)
                .collect(),
        );
        to_js(&DeltaView {
            changed,
            removed: Vec::new(),
            backlinks,
        })
    }
}
