// SPDX-License-Identifier: MPL-2.0
/// Update function — handles all state transitions.
///
/// All notebook mutations delegate to the Rust core (WasmStore); the
/// model's notebook is a read model patched with what the core returns.
/// The dicts are mutated in place — a new notebook record is produced per
/// transition so React re-renders, but older model values must not be
/// treated as immutable history.

open Types
open Model
open Msg

%%private(
  let deleteKey: (Js.Dict.t<'a>, string) => unit = %raw(`(dict, key) => { delete dict[key] }`)
)

/// Upsert a single note view returned by the core.
let setNote = (notebook: notebook, note: note): notebook => {
  Js.Dict.set(notebook.notes, note.id, note)
  {...notebook, modifiedAt: note.modifiedAt}
}

/// Apply a topology delta returned by the core.
let applyDelta = (notebook: notebook, delta: WasmStore.delta): notebook => {
  delta.changed->Array.forEach(note => Js.Dict.set(notebook.notes, note.id, note))
  delta.removed->Array.forEach(id => {
    deleteKey(notebook.notes, id)
    deleteKey(notebook.backlinks, id)
  })
  Js.Dict.entries(delta.backlinks)->Array.forEach(((id, sources)) =>
    Js.Dict.set(notebook.backlinks, id, sources)
  )
  // Spread with a no-op override: a fresh record identity so React re-renders.
  {...notebook, name: notebook.name}
}

let patchNote = (model: model, result: result<note, string>): model =>
  switch result {
  | Ok(note) => {...model, notebook: setNote(model.notebook, note), dirty: true}
  | Error(message) => {...model, error: Some(message)}
  }

/// The main update function
let rec update = (model: model, msg: msg): model => {
  switch msg {
  // Note CRUD
  | CreateNote =>
    switch WasmStore.createNote("New Note") {
    | Ok(note) => {
        ...model,
        notebook: setNote(model.notebook, note),
        selection: SingleNote(note.id),
        editingNote: Some(note.id),
        dirty: true,
      }
    | Error(message) => {...model, error: Some(message)}
    }

  | CreateNoteAt(position) =>
    switch WasmStore.createNoteAt("New Note", position.x, position.y) {
    | Ok(note) => {
        ...model,
        notebook: setNote(model.notebook, note),
        selection: SingleNote(note.id),
        editingNote: Some(note.id),
        dirty: true,
      }
    | Error(message) => {...model, error: Some(message)}
    }

  | DeleteNote(id) =>
    switch WasmStore.deleteNote(id) {
    | Ok(delta) => {
        ...model,
        notebook: applyDelta(model.notebook, delta),
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
    | Error(message) => {...model, error: Some(message)}
    }

  | DeleteSelectedNotes =>
    // Guarded here rather than in the keyboard handler so the check always
    // sees current state (the handler is registered once and would capture a
    // stale model).
    switch model.editingNote {
    | Some(_) => model
    | None =>
      switch model.selection {
      | NoSelection => model
      | SingleNote(id) => update(model, DeleteNote(id))
      | MultipleNotes(ids) => ids->Array.reduce(model, (m, id) => update(m, DeleteNote(id)))
      }
    }

  // Note editing
  | UpdateNoteTitle(id, title) => patchNote(model, WasmStore.updateTitle(id, title))

  | UpdateNoteContent(id, content) => patchNote(model, WasmStore.updateContent(id, content))

  | StartEditingNote(id) => {...model, editingNote: Some(id)}

  | StopEditingNote => {...model, editingNote: None}

  // Note positioning
  | MoveNote(id, position) => patchNote(model, WasmStore.moveNote(id, position.x, position.y))

  | ResizeNote(id, width, height) => patchNote(model, WasmStore.resizeNote(id, width, height))

  // Links
  | LinkNotes(fromId, toId) =>
    if fromId == toId {
      model
    } else {
      switch WasmStore.link(fromId, toId) {
      | Ok(delta) => {...model, notebook: applyDelta(model.notebook, delta), dirty: true}
      | Error(message) => {...model, error: Some(message)}
      }
    }

  | UnlinkNotes(fromId, toId) =>
    switch WasmStore.unlink(fromId, toId) {
    | Ok(delta) => {...model, notebook: applyDelta(model.notebook, delta), dirty: true}
    | Error(message) => {...model, error: Some(message)}
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
      searchResults: query == "" ? [] : WasmStore.search(query),
    }

  | ClearSearch => {...model, searchQuery: "", searchResults: []}

  // File operations
  | NewNotebook => {
      ...initial(),
      notebook: WasmStore.reset("Untitled Notebook"),
      viewMode: model.viewMode,
      sidebarOpen: model.sidebarOpen,
    }

  | SaveNotebook =>
    switch WasmStore.toJson() {
    | Ok(json) => {
        Persist.saveToFile(model.notebook.name, json)
        {...model, dirty: false}
      }
    | Error(message) => {...model, error: Some(message)}
    }

  | SaveNotebookAs(name) =>
    switch WasmStore.toJson() {
    | Ok(json) => {
        Persist.saveToFile(name, json)
        {...model, dirty: false}
      }
    | Error(message) => {...model, error: Some(message)}
    }

  | LoadNotebook(_path) => {
      // Async: the picker resolves after update() returns; the result comes
      // back through Dispatcher as NotebookLoaded / SetError.
      Persist.openFile()
      ->Promise.thenResolve(content =>
        switch content {
        | Some(json) =>
          switch WasmStore.loadFromJson(json) {
          | Ok(snapshot) => Dispatcher.dispatch(NotebookLoaded(snapshot))
          | Error(message) => Dispatcher.dispatch(SetError(message))
          }
        | None => ()
        }
      )
      ->ignore
      model
    }

  | NotebookLoaded(notebook) => {
      ...model,
      notebook,
      dirty: false,
      selection: NoSelection,
      editingNote: None,
      searchQuery: "",
      searchResults: [],
      error: None,
    }

  | NotebookSaved => {...model, dirty: false}

  // Errors
  | SetError(error) => {...model, error: Some(error)}

  | ClearError => {...model, error: None}

  | NoOp => model
  }
}
