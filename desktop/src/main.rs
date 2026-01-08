// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nexia desktop application entry point

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nexia_core::{Notebook, Note, NoteId, Storage, storage::JsonStorage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Application state shared across commands
struct AppState {
    notebook: Mutex<Notebook>,
    file_path: Mutex<Option<PathBuf>>,
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

/// Response wrapper for commands
#[derive(Serialize)]
struct CommandResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> CommandResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Create a new note
#[tauri::command]
fn create_note(state: State<AppState>, title: String) -> CommandResponse<Note> {
    let mut notebook = state.notebook.lock().unwrap();
    let note = Note::new(title);
    let id = note.id;
    notebook.add_note(note);

    match notebook.get_note(&id) {
        Some(note) => CommandResponse::ok(note.clone()),
        None => CommandResponse::err("Failed to create note"),
    }
}

/// Get a note by ID
#[tauri::command]
fn get_note(state: State<AppState>, id: String) -> CommandResponse<Note> {
    let notebook = state.notebook.lock().unwrap();
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid note ID"),
    };

    match notebook.get_note(&uuid) {
        Some(note) => CommandResponse::ok(note.clone()),
        None => CommandResponse::err("Note not found"),
    }
}

/// Get all notes
#[tauri::command]
fn get_all_notes(state: State<AppState>) -> CommandResponse<Vec<Note>> {
    let notebook = state.notebook.lock().unwrap();
    let notes: Vec<Note> = notebook.all_notes().cloned().collect();
    CommandResponse::ok(notes)
}

/// Update a note's title
#[tauri::command]
fn update_note_title(state: State<AppState>, id: String, title: String) -> CommandResponse<Note> {
    let mut notebook = state.notebook.lock().unwrap();
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid note ID"),
    };

    if let Some(note) = notebook.get_note_mut(&uuid) {
        note.title = title;
        note.touch();
        CommandResponse::ok(note.clone())
    } else {
        CommandResponse::err("Note not found")
    }
}

/// Update a note's content
#[tauri::command]
fn update_note_content(state: State<AppState>, id: String, content: String) -> CommandResponse<Note> {
    let mut notebook = state.notebook.lock().unwrap();
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid note ID"),
    };

    if let Some(note) = notebook.get_note_mut(&uuid) {
        note.content = content;
        note.touch();
        CommandResponse::ok(note.clone())
    } else {
        CommandResponse::err("Note not found")
    }
}

/// Delete a note
#[tauri::command]
fn delete_note(state: State<AppState>, id: String) -> CommandResponse<()> {
    let mut notebook = state.notebook.lock().unwrap();
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid note ID"),
    };

    match notebook.remove_note(&uuid) {
        Some(_) => CommandResponse::ok(()),
        None => CommandResponse::err("Note not found"),
    }
}

/// Link two notes
#[tauri::command]
fn link_notes(state: State<AppState>, from_id: String, to_id: String) -> CommandResponse<()> {
    let mut notebook = state.notebook.lock().unwrap();

    let from_uuid = match uuid::Uuid::parse_str(&from_id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid source note ID"),
    };

    let to_uuid = match uuid::Uuid::parse_str(&to_id) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResponse::err("Invalid target note ID"),
    };

    match notebook.link_notes(from_uuid, to_uuid) {
        Ok(_) => CommandResponse::ok(()),
        Err(e) => CommandResponse::err(e.to_string()),
    }
}

/// Search notes
#[tauri::command]
fn search_notes(state: State<AppState>, query: String) -> CommandResponse<Vec<Note>> {
    let notebook = state.notebook.lock().unwrap();
    let results: Vec<Note> = notebook.search(&query).into_iter().cloned().collect();
    CommandResponse::ok(results)
}

/// Save notebook to file
#[tauri::command]
fn save_notebook(state: State<AppState>, path: Option<String>) -> CommandResponse<String> {
    let notebook = state.notebook.lock().unwrap();
    let mut file_path = state.file_path.lock().unwrap();

    let save_path = match path {
        Some(p) => {
            let path = PathBuf::from(&p);
            *file_path = Some(path.clone());
            path
        }
        None => match file_path.as_ref() {
            Some(p) => p.clone(),
            None => return CommandResponse::err("No file path specified"),
        },
    };

    match state.storage.save(&notebook, &save_path) {
        Ok(_) => CommandResponse::ok(save_path.display().to_string()),
        Err(e) => CommandResponse::err(e.to_string()),
    }
}

/// Load notebook from file
#[tauri::command]
fn load_notebook(state: State<AppState>, path: String) -> CommandResponse<Notebook> {
    let path = PathBuf::from(&path);

    match state.storage.load(&path) {
        Ok(loaded) => {
            let mut notebook = state.notebook.lock().unwrap();
            let mut file_path = state.file_path.lock().unwrap();
            *notebook = loaded.clone();
            *file_path = Some(path);
            CommandResponse::ok(loaded)
        }
        Err(e) => CommandResponse::err(e.to_string()),
    }
}

/// New notebook
#[tauri::command]
fn new_notebook(state: State<AppState>, name: String) -> CommandResponse<()> {
    let mut notebook = state.notebook.lock().unwrap();
    let mut file_path = state.file_path.lock().unwrap();
    *notebook = Notebook::new(name);
    *file_path = None;
    CommandResponse::ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            create_note,
            get_note,
            get_all_notes,
            update_note_title,
            update_note_content,
            delete_note,
            link_notes,
            search_notes,
            save_notebook,
            load_notebook,
            new_notebook,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
