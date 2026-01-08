// SPDX-License-Identifier: AGPL-3.0-or-later
/// Application model - single source of truth

open Types

/// The complete application state
type model = {
  /// All notes in the notebook
  notebook: notebook,
  /// Current view mode
  viewMode: viewMode,
  /// Currently selected notes
  selection: selection,
  /// Canvas viewport state
  viewport: viewport,
  /// Search query
  searchQuery: string,
  /// Search results (note IDs)
  searchResults: array<noteId>,
  /// Currently editing note (for inline edit)
  editingNote: option<noteId>,
  /// Sidebar visibility
  sidebarOpen: bool,
  /// File path of current notebook
  filePath: option<string>,
  /// Unsaved changes flag
  dirty: bool,
  /// Error message to display
  error: option<string>,
}

/// Create an empty notebook
let emptyNotebook = (): notebook => {
  let now = Js.Date.toISOString(Js.Date.make())
  {
    notes: Js.Dict.empty(),
    backlinks: Js.Dict.empty(),
    name: "Untitled Notebook",
    createdAt: now,
    modifiedAt: now,
  }
}

/// Initial application state
let initial = (): model => {
  notebook: emptyNotebook(),
  viewMode: ListView,
  selection: NoSelection,
  viewport: Viewport.initial(),
  searchQuery: "",
  searchResults: [],
  editingNote: None,
  sidebarOpen: true,
  filePath: None,
  dirty: false,
  error: None,
}

/// Get a note by ID from the model
let getNote = (model: model, id: noteId): option<note> => {
  Js.Dict.get(model.notebook.notes, id)
}

/// Get all notes as an array
let allNotes = (model: model): array<note> => {
  Js.Dict.values(model.notebook.notes)
}

/// Get backlinks for a note
let getBacklinks = (model: model, id: noteId): array<noteId> => {
  switch Js.Dict.get(model.notebook.backlinks, id) {
  | Some(links) => links
  | None => []
  }
}

/// Count total notes
let noteCount = (model: model): int => {
  Js.Dict.keys(model.notebook.notes)->Array.length
}
