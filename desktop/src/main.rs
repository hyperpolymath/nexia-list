// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
//! Nexia-List desktop application entry point — Gossamer webview shell.
//!
//! All 11 note-management commands are registered via `gossamer_rs::App::command()`.
//! Each handler receives a `serde_json::Value` payload and returns
//! `Result<serde_json::Value, String>`. Business logic is identical to the
//! former Tauri implementation; only the shell layer changed.

use gossamer_rs::App;
use nexia_core::{Notebook, Note, Storage, storage::JsonStorage};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

// =============================================================================
// Shared application state
// =============================================================================

/// Application state shared across all command handlers via `Arc`-like
/// interior mutability (Mutex). Gossamer dispatches commands on a single
/// thread, so contention is minimal.
struct AppState {
    /// The currently loaded notebook.
    notebook: Mutex<Notebook>,
    /// Path to the on-disk file (None if unsaved).
    file_path: Mutex<Option<PathBuf>>,
    /// JSON storage backend for save/load operations.
    storage: JsonStorage,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            notebook: Mutex::new(Notebook::new("Untitled")),
            file_path: Mutex::new(None),
            storage: JsonStorage::new(),
        }
    }
}

// =============================================================================
// Command response wrapper
// =============================================================================

/// Uniform envelope returned by every command.
/// Frontend code expects `{ success, data?, error? }`.
#[derive(Serialize)]
struct CommandResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl<T: Serialize> CommandResponse<T> {
    /// Successful response containing `data`.
    fn ok(data: T) -> serde_json::Value {
        serde_json::to_value(CommandResponse {
            success: true,
            data: Some(data),
            error: None,
        })
        .unwrap_or_else(|e| serde_json::json!({ "success": false, "error": e.to_string() }))
    }

    /// Error response with a human-readable message.
    fn err(message: impl Into<String>) -> serde_json::Value {
        serde_json::to_value(CommandResponse::<()> {
            success: false,
            data: None,
            error: Some(message.into()),
        })
        .unwrap_or_else(|e| serde_json::json!({ "success": false, "error": e.to_string() }))
    }
}

// =============================================================================
// UUID parsing helper
// =============================================================================

/// Parse a UUID string from the JSON payload, returning a Gossamer-compatible error.
fn parse_uuid(payload: &serde_json::Value, key: &str) -> Result<uuid::Uuid, String> {
    let id_str = payload[key]
        .as_str()
        .ok_or_else(|| format!("missing or invalid '{key}' field"))?;
    uuid::Uuid::parse_str(id_str).map_err(|_| format!("Invalid {key}: {id_str}"))
}

// =============================================================================
// Entry point
// =============================================================================

fn main() -> Result<(), gossamer_rs::Error> {
    // Shared state wrapped in Arc for multi-handler access.
    // Gossamer command closures require 'static + Send, so Arc is necessary.
    let state = std::sync::Arc::new(AppState::default());

    // Create the Gossamer webview window matching gossamer.conf.json dimensions.
    let mut app = App::new("Nexia-List", 1200, 800)?;

    // -- create_note ----------------------------------------------------------
    {
        let st = state.clone();
        app.command("create_note", move |payload| {
            let title = payload["title"]
                .as_str()
                .unwrap_or("Untitled")
                .to_string();
            let mut notebook = st.notebook.lock().unwrap();
            let note = Note::new(title);
            let id = note.id;
            notebook.add_note(note);

            match notebook.get_note(&id) {
                Some(note) => Ok(CommandResponse::<Note>::ok(note.clone())),
                None => Ok(CommandResponse::<Note>::err("Failed to create note")),
            }
        });
    }

    // -- get_note -------------------------------------------------------------
    {
        let st = state.clone();
        app.command("get_note", move |payload| {
            let uuid = parse_uuid(&payload, "id")?;
            let notebook = st.notebook.lock().unwrap();
            match notebook.get_note(&uuid) {
                Some(note) => Ok(CommandResponse::<Note>::ok(note.clone())),
                None => Ok(CommandResponse::<Note>::err("Note not found")),
            }
        });
    }

    // -- get_all_notes --------------------------------------------------------
    {
        let st = state.clone();
        app.command("get_all_notes", move |_payload| {
            let notebook = st.notebook.lock().unwrap();
            let notes: Vec<Note> = notebook.all_notes().cloned().collect();
            Ok(CommandResponse::<Vec<Note>>::ok(notes))
        });
    }

    // -- update_note_title ----------------------------------------------------
    {
        let st = state.clone();
        app.command("update_note_title", move |payload| {
            let uuid = parse_uuid(&payload, "id")?;
            let title = payload["title"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let mut notebook = st.notebook.lock().unwrap();
            if let Some(note) = notebook.get_note_mut(&uuid) {
                note.title = title;
                note.touch();
                Ok(CommandResponse::<Note>::ok(note.clone()))
            } else {
                Ok(CommandResponse::<Note>::err("Note not found"))
            }
        });
    }

    // -- update_note_content --------------------------------------------------
    {
        let st = state.clone();
        app.command("update_note_content", move |payload| {
            let uuid = parse_uuid(&payload, "id")?;
            let content = payload["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let mut notebook = st.notebook.lock().unwrap();
            if let Some(note) = notebook.get_note_mut(&uuid) {
                note.content = content;
                note.touch();
                Ok(CommandResponse::<Note>::ok(note.clone()))
            } else {
                Ok(CommandResponse::<Note>::err("Note not found"))
            }
        });
    }

    // -- delete_note ----------------------------------------------------------
    {
        let st = state.clone();
        app.command("delete_note", move |payload| {
            let uuid = parse_uuid(&payload, "id")?;
            let mut notebook = st.notebook.lock().unwrap();
            match notebook.remove_note(&uuid) {
                Some(_) => Ok(CommandResponse::<()>::ok(())),
                None => Ok(CommandResponse::<()>::err("Note not found")),
            }
        });
    }

    // -- link_notes -----------------------------------------------------------
    {
        let st = state.clone();
        app.command("link_notes", move |payload| {
            let from_uuid = parse_uuid(&payload, "from_id")?;
            let to_uuid = parse_uuid(&payload, "to_id")?;
            let mut notebook = st.notebook.lock().unwrap();
            match notebook.link_notes(from_uuid, to_uuid) {
                Ok(_) => Ok(CommandResponse::<()>::ok(())),
                Err(e) => Ok(CommandResponse::<()>::err(e.to_string())),
            }
        });
    }

    // -- search_notes ---------------------------------------------------------
    {
        let st = state.clone();
        app.command("search_notes", move |payload| {
            let query = payload["query"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let notebook = st.notebook.lock().unwrap();
            let results: Vec<Note> = notebook.search(&query).into_iter().cloned().collect();
            Ok(CommandResponse::<Vec<Note>>::ok(results))
        });
    }

    // -- save_notebook --------------------------------------------------------
    {
        let st = state.clone();
        app.command("save_notebook", move |payload| {
            let notebook = st.notebook.lock().unwrap();
            let mut file_path = st.file_path.lock().unwrap();

            let save_path = match payload["path"].as_str() {
                Some(p) => {
                    let path = PathBuf::from(p);
                    *file_path = Some(path.clone());
                    path
                }
                None => match file_path.as_ref() {
                    Some(p) => p.clone(),
                    None => {
                        return Ok(CommandResponse::<String>::err("No file path specified"));
                    }
                },
            };

            match st.storage.save(&notebook, &save_path) {
                Ok(_) => Ok(CommandResponse::<String>::ok(
                    save_path.display().to_string(),
                )),
                Err(e) => Ok(CommandResponse::<String>::err(e.to_string())),
            }
        });
    }

    // -- load_notebook --------------------------------------------------------
    {
        let st = state.clone();
        app.command("load_notebook", move |payload| {
            let path_str = payload["path"]
                .as_str()
                .ok_or("missing 'path' field")?;
            let path = PathBuf::from(path_str);

            match st.storage.load(&path) {
                Ok(loaded) => {
                    let mut notebook = st.notebook.lock().unwrap();
                    let mut file_path = st.file_path.lock().unwrap();
                    *notebook = loaded.clone();
                    *file_path = Some(path);
                    Ok(CommandResponse::<Notebook>::ok(loaded))
                }
                Err(e) => Ok(CommandResponse::<Notebook>::err(e.to_string())),
            }
        });
    }

    // -- new_notebook ---------------------------------------------------------
    {
        let st = state.clone();
        app.command("new_notebook", move |payload| {
            let name = payload["name"]
                .as_str()
                .unwrap_or("Untitled")
                .to_string();
            let mut notebook = st.notebook.lock().unwrap();
            let mut file_path = st.file_path.lock().unwrap();
            *notebook = Notebook::new(name);
            *file_path = None;
            Ok(CommandResponse::<()>::ok(()))
        });
    }

    // Navigate to the frontend dist directory (served by Gossamer).
    // gossamer.conf.json specifies frontendDist: "../web/dist"
    app.navigate("/")?;

    // Run the event loop — blocks until the window is closed.
    app.run();
    Ok(())
}
