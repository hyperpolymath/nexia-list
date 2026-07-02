// SPDX-License-Identifier: MPL-2.0
/// Bindings to the WASM-compiled Rust core plus the current-notebook handle.
///
/// This module is the seam between the TEA UI and the engine: Update.res
/// mutates through it and patches its read model with the returned values.
/// A GossamerStore implementing the same operations over the desktop bridge
/// can replace it in a webview shell without UI changes.

open Types

type t

type delta = {
  changed: array<note>,
  removed: array<noteId>,
  backlinks: Dict.t<array<noteId>>,
}

@module("../../../web/wasm/nexia_core.js")
external initWasm: string => promise<unit> = "default"

@module("../../../web/wasm/nexia_core.js") @new
external makeNotebook: string => t = "WasmNotebook"

@module("../../../web/wasm/nexia_core.js") @scope("WasmNotebook")
external fromJsonRaw: string => t = "from_json"

@send external createNoteRaw: (t, string) => note = "create_note"
@send external createNoteAtRaw: (t, string, float, float) => note = "create_note_at"
@send external updateTitleRaw: (t, noteId, string) => note = "update_title"
@send external updateContentRaw: (t, noteId, string) => note = "update_content"
@send external moveNoteRaw: (t, noteId, float, float) => note = "move_note"
@send external resizeNoteRaw: (t, noteId, float, float) => note = "resize_note"
@send external deleteNoteRaw: (t, noteId) => delta = "delete_note"
@send external linkRaw: (t, noteId, noteId) => delta = "link"
@send external unlinkRaw: (t, noteId, noteId) => delta = "unlink"
@send external searchRaw: (t, string) => array<noteId> = "search"
@send external snapshotRaw: t => notebook = "snapshot"
@send external toJsonRaw: t => string = "to_json"

let current: ref<option<t>> = ref(None)

exception StoreNotInitialized

let instance = (): t =>
  switch current.contents {
  | Some(store) => store
  | None => raise(StoreNotInitialized)
  }

/// Load the wasm module and start with an empty notebook.
let init = async (wasmUrl: string) => {
  await initWasm(wasmUrl)
  current := Some(makeNotebook("Untitled Notebook"))
}

/// Replace the current notebook with a fresh one; returns its snapshot.
let reset = (name: string): notebook => {
  let nb = makeNotebook(name)
  current := Some(nb)
  snapshotRaw(nb)
}

let errorMessage = (e: exn, fallback: string): string =>
  switch e {
  | Exn.Error(jsError) => jsError->Exn.message->Option.getOr(fallback)
  | _ => fallback
  }

/// Replace the current notebook from on-disk JSON; returns its snapshot.
let loadFromJson = (json: string): result<notebook, string> =>
  try {
    let nb = fromJsonRaw(json)
    current := Some(nb)
    Ok(snapshotRaw(nb))
  } catch {
  | e => Error(errorMessage(e, "Could not load notebook"))
  }

let snapshot = (): notebook => snapshotRaw(instance())

let toJson = (): result<string, string> =>
  try Ok(toJsonRaw(instance())) catch {
  | e => Error(errorMessage(e, "Could not serialize notebook"))
  }

let tryNote = (operation: unit => note): result<note, string> =>
  try Ok(operation()) catch {
  | e => Error(errorMessage(e, "Note operation failed"))
  }

let tryDelta = (operation: unit => delta): result<delta, string> =>
  try Ok(operation()) catch {
  | e => Error(errorMessage(e, "Link operation failed"))
  }

let createNote = (title: string) => tryNote(() => createNoteRaw(instance(), title))
let createNoteAt = (title: string, x: float, y: float) =>
  tryNote(() => createNoteAtRaw(instance(), title, x, y))
let updateTitle = (id: noteId, title: string) => tryNote(() => updateTitleRaw(instance(), id, title))
let updateContent = (id: noteId, content: string) =>
  tryNote(() => updateContentRaw(instance(), id, content))
let moveNote = (id: noteId, x: float, y: float) => tryNote(() => moveNoteRaw(instance(), id, x, y))
let resizeNote = (id: noteId, width: float, height: float) =>
  tryNote(() => resizeNoteRaw(instance(), id, width, height))

let deleteNote = (id: noteId) => tryDelta(() => deleteNoteRaw(instance(), id))
let link = (fromId: noteId, toId: noteId) => tryDelta(() => linkRaw(instance(), fromId, toId))
let unlink = (fromId: noteId, toId: noteId) => tryDelta(() => unlinkRaw(instance(), fromId, toId))

let search = (query: string): array<noteId> => searchRaw(instance(), query)
