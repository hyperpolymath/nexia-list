// SPDX-License-Identifier: MPL-2.0
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
  // Keyboard canvas navigation
  | NavigateCanvas(direction)
  | NudgeSelectedNote(direction)
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
  // Import / export
  | ExportMarkdown
  | ExportOpml
  | ImportVault
  // Errors
  | SetError(string)
  | ClearError
  // No-op
  | NoOp
