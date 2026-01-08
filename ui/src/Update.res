// SPDX-License-Identifier: AGPL-3.0-or-later
/// Update function - handles all state transitions

open Types
open Model
open Msg

/// Helper to update a note in the notebook
let updateNoteInNotebook = (notebook: notebook, id: noteId, updater: note => note): notebook => {
  switch Js.Dict.get(notebook.notes, id) {
  | Some(note) =>
    let now = Js.Date.toISOString(Js.Date.make())
    let updatedNote = {...updater(note), modifiedAt: now}
    let newNotes = Js.Dict.fromArray(
      Js.Dict.entries(notebook.notes)->Array.map(((k, v)) =>
        if k == id {
          (k, updatedNote)
        } else {
          (k, v)
        }
      ),
    )
    {...notebook, notes: newNotes, modifiedAt: now}
  | None => notebook
  }
}

/// Add a note to the notebook
let addNoteToNotebook = (notebook: notebook, note: note): notebook => {
  let now = Js.Date.toISOString(Js.Date.make())
  let newNotes = Js.Dict.fromArray(
    Array.concat(Js.Dict.entries(notebook.notes), [(note.id, note)]),
  )
  {...notebook, notes: newNotes, modifiedAt: now}
}

/// Remove a note from the notebook
let removeNoteFromNotebook = (notebook: notebook, id: noteId): notebook => {
  let now = Js.Date.toISOString(Js.Date.make())
  let newNotes = Js.Dict.fromArray(
    Js.Dict.entries(notebook.notes)->Array.filter(((k, _)) => k != id),
  )
  // Also remove from backlinks and remove links to this note
  let newBacklinks = Js.Dict.fromArray(
    Js.Dict.entries(notebook.backlinks)
    ->Array.filter(((k, _)) => k != id)
    ->Array.map(((k, v)) => (k, v->Array.filter(linkId => linkId != id))),
  )
  {...notebook, notes: newNotes, backlinks: newBacklinks, modifiedAt: now}
}

/// Add a link between notes
let addLinkToNotebook = (notebook: notebook, fromId: noteId, toId: noteId): notebook => {
  // Add forward link
  let notebook = updateNoteInNotebook(notebook, fromId, note => {
    if !Array.includes(note.links, toId) {
      {...note, links: Array.concat(note.links, [toId])}
    } else {
      note
    }
  })

  // Add backlink
  let currentBacklinks = switch Js.Dict.get(notebook.backlinks, toId) {
  | Some(links) => links
  | None => []
  }
  if !Array.includes(currentBacklinks, fromId) {
    let newBacklinks = Js.Dict.fromArray(
      Array.concat(
        Js.Dict.entries(notebook.backlinks)->Array.filter(((k, _)) => k != toId),
        [(toId, Array.concat(currentBacklinks, [fromId]))],
      ),
    )
    {...notebook, backlinks: newBacklinks}
  } else {
    notebook
  }
}

/// Remove a link between notes
let removeLinkFromNotebook = (notebook: notebook, fromId: noteId, toId: noteId): notebook => {
  // Remove forward link
  let notebook = updateNoteInNotebook(notebook, fromId, note => {
    {...note, links: note.links->Array.filter(id => id != toId)}
  })

  // Remove backlink
  let currentBacklinks = switch Js.Dict.get(notebook.backlinks, toId) {
  | Some(links) => links
  | None => []
  }
  let newBacklinks = Js.Dict.fromArray(
    Array.concat(
      Js.Dict.entries(notebook.backlinks)->Array.filter(((k, _)) => k != toId),
      [(toId, currentBacklinks->Array.filter(id => id != fromId))],
    ),
  )
  {...notebook, backlinks: newBacklinks}
}

/// Simple search implementation
let searchNotes = (notebook: notebook, query: string): array<noteId> => {
  if query == "" {
    []
  } else {
    let queryLower = String.toLowerCase(query)
    Js.Dict.entries(notebook.notes)
    ->Array.filter(((_, note)) => {
      String.toLowerCase(note.title)->String.includes(queryLower) ||
        String.toLowerCase(note.content)->String.includes(queryLower)
    })
    ->Array.map(((id, _)) => id)
  }
}

/// The main update function
let update = (model: model, msg: msg): model => {
  switch msg {
  // Note CRUD
  | CreateNote =>
    let note = Note.make(~title="New Note")
    {
      ...model,
      notebook: addNoteToNotebook(model.notebook, note),
      selection: SingleNote(note.id),
      editingNote: Some(note.id),
      dirty: true,
    }

  | CreateNoteAt(position) =>
    let note = Note.make(~title="New Note")->Note.withPosition(position.x, position.y)
    {
      ...model,
      notebook: addNoteToNotebook(model.notebook, note),
      selection: SingleNote(note.id),
      editingNote: Some(note.id),
      dirty: true,
    }

  | DeleteNote(id) => {
      ...model,
      notebook: removeNoteFromNotebook(model.notebook, id),
      selection: switch model.selection {
      | SingleNote(selectedId) if selectedId == id => NoSelection
      | MultipleNotes(ids) => {
          let remaining = ids->Array.filter(i => i != id)
          switch remaining {
          | [] => NoSelection
          | [single] => SingleNote(single)
          | multiple => MultipleNotes(multiple)
          }
        }
      | other => other
      },
      editingNote: switch model.editingNote {
      | Some(editId) if editId == id => None
      | other => other
      },
      dirty: true,
    }

  | DeleteSelectedNotes =>
    switch model.selection {
    | NoSelection => model
    | SingleNote(id) => update(model, DeleteNote(id))
    | MultipleNotes(ids) =>
      ids->Array.reduce(model, (m, id) => update(m, DeleteNote(id)))
    }

  // Note editing
  | UpdateNoteTitle(id, title) => {
      ...model,
      notebook: updateNoteInNotebook(model.notebook, id, note => {...note, title}),
      dirty: true,
    }

  | UpdateNoteContent(id, content) => {
      ...model,
      notebook: updateNoteInNotebook(model.notebook, id, note => {...note, content}),
      dirty: true,
    }

  | StartEditingNote(id) => {...model, editingNote: Some(id)}

  | StopEditingNote => {...model, editingNote: None}

  // Note positioning
  | MoveNote(id, position) => {
      ...model,
      notebook: updateNoteInNotebook(model.notebook, id, note => {
        {...note, position: Some(position)}
      }),
      dirty: true,
    }

  | ResizeNote(id, width, height) => {
      ...model,
      notebook: updateNoteInNotebook(model.notebook, id, note => {
        {...note, size: Some((width, height))}
      }),
      dirty: true,
    }

  // Links
  | LinkNotes(fromId, toId) =>
    if fromId == toId {
      model
    } else {
      {
        ...model,
        notebook: addLinkToNotebook(model.notebook, fromId, toId),
        dirty: true,
      }
    }

  | UnlinkNotes(fromId, toId) => {
      ...model,
      notebook: removeLinkFromNotebook(model.notebook, fromId, toId),
      dirty: true,
    }

  // Selection
  | SelectNote(id) => {...model, selection: SingleNote(id)}

  | AddToSelection(id) =>
    switch model.selection {
    | NoSelection => {...model, selection: SingleNote(id)}
    | SingleNote(existing) =>
      if existing == id {
        model
      } else {
        {...model, selection: MultipleNotes([existing, id])}
      }
    | MultipleNotes(ids) =>
      if Array.includes(ids, id) {
        model
      } else {
        {...model, selection: MultipleNotes(Array.concat(ids, [id]))}
      }
    }

  | ClearSelection => {...model, selection: NoSelection}

  | SelectAll => {
      let allIds = Js.Dict.keys(model.notebook.notes)
      {
        ...model,
        selection: switch allIds {
        | [] => NoSelection
        | [single] => SingleNote(single)
        | multiple => MultipleNotes(multiple)
        },
      }
    }

  // View
  | SetViewMode(mode) => {...model, viewMode: mode}

  | ToggleSidebar => {...model, sidebarOpen: !model.sidebarOpen}

  // Canvas
  | PanCanvas(dx, dy) => {
      ...model,
      viewport: {
        ...model.viewport,
        offsetX: model.viewport.offsetX +. dx,
        offsetY: model.viewport.offsetY +. dy,
      },
    }

  | ZoomCanvas(factor) => {
      let newZoom = model.viewport.zoom *. factor
      let clampedZoom = Js.Math.max_float(0.1, Js.Math.min_float(5.0, newZoom))
      {...model, viewport: {...model.viewport, zoom: clampedZoom}}
    }

  | ResetViewport => {...model, viewport: Viewport.initial()}

  // Search
  | SetSearchQuery(query) => {
      ...model,
      searchQuery: query,
      searchResults: searchNotes(model.notebook, query),
    }

  | ClearSearch => {...model, searchQuery: "", searchResults: []}

  // File operations
  | NewNotebook => {
      ...initial(),
      viewMode: model.viewMode,
      sidebarOpen: model.sidebarOpen,
    }

  | SaveNotebook => model // Handled by command in full TEA setup

  | SaveNotebookAs(_path) => model // Handled by command

  | LoadNotebook(_path) => model // Handled by command

  | NotebookLoaded(notebook) => {
      ...model,
      notebook,
      dirty: false,
      selection: NoSelection,
      editingNote: None,
    }

  | NotebookSaved => {...model, dirty: false}

  // Errors
  | SetError(error) => {...model, error: Some(error)}

  | ClearError => {...model, error: None}

  | NoOp => model
  }
}
