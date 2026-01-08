// SPDX-License-Identifier: AGPL-3.0-or-later
/// All application messages

open Types

/// Messages that can be sent to update the model
type msg =
  // Note CRUD
  | CreateNote
  | CreateNoteAt(point2D)
  | DeleteNote(noteId)
  | DeleteSelectedNotes
  // Note editing
  | UpdateNoteTitle(noteId, string)
  | UpdateNoteContent(noteId, string)
  | StartEditingNote(noteId)
  | StopEditingNote
  // Note positioning
  | MoveNote(noteId, point2D)
  | ResizeNote(noteId, float, float)
  // Links
  | LinkNotes(noteId, noteId)
  | UnlinkNotes(noteId, noteId)
  // Selection
  | SelectNote(noteId)
  | AddToSelection(noteId)
  | ClearSelection
  | SelectAll
  // View
  | SetViewMode(viewMode)
  | ToggleSidebar
  // Canvas
  | PanCanvas(float, float)
  | ZoomCanvas(float)
  | ResetViewport
  // Search
  | SetSearchQuery(string)
  | ClearSearch
  // File operations
  | NewNotebook
  | SaveNotebook
  | SaveNotebookAs(string)
  | LoadNotebook(string)
  | NotebookLoaded(notebook)
  | NotebookSaved
  // Errors
  | SetError(string)
  | ClearError
  // No-op
  | NoOp
