// SPDX-License-Identifier: MPL-2.0
/// Pure TEA update tests, run against the real WASM core.
/// The JS wrapper (update.test.js) initializes the wasm module before
/// calling runAll; every assertion failure raises with its label.

open Types

exception AssertionFailed(string)

let check = (cond: bool, label: string) =>
  if !cond {
    raise(AssertionFailed(label))
  }

let selectedId = (model: Model.model, label: string): noteId =>
  switch model.selection {
  | SingleNote(id) => id
  | _ => raise(AssertionFailed(label))
  }

let runAll = () => {
  let snapshot = WasmStore.reset("Test")
  let model = {...Model.initial(), notebook: snapshot}

  // Create
  let model = Update.update(model, Msg.CreateNote)
  check(Model.noteCount(model) == 1, "create adds a note")
  let alpha = selectedId(model, "note selected after create")
  check(model.editingNote == Some(alpha), "editing starts after create")
  check(model.dirty, "dirty after create")

  // Ids come from the core: UUID shape, not a random float
  check(String.length(alpha) == 36, "core-issued UUID id")

  // Edit through the core
  let model = Update.update(model, Msg.UpdateNoteTitle(alpha, "Alpha"))
  check((Model.getNote(model, alpha)->Option.getExn).title == "Alpha", "title updated")
  let model = Update.update(model, Msg.UpdateNoteContent(alpha, "Spatial hypertext"))
  check(
    (Model.getNote(model, alpha)->Option.getExn).content == "Spatial hypertext",
    "content updated",
  )

  // Second note, then link
  let model = Update.update(model, Msg.CreateNote)
  let beta = selectedId(model, "note selected after second create")
  let model = Update.update(model, Msg.LinkNotes(alpha, beta))
  check(
    (Model.getNote(model, alpha)->Option.getExn).links->Array.includes(beta),
    "forward link recorded",
  )
  check(Model.getBacklinks(model, beta)->Array.includes(alpha), "backlink recorded")

  // Self-link is a no-op
  let selfLinked = Update.update(model, Msg.LinkNotes(alpha, alpha))
  check(
    (Model.getNote(selfLinked, alpha)->Option.getExn).links->Array.length == 1,
    "self link ignored",
  )

  // Unlink
  let unlinked = Update.update(model, Msg.UnlinkNotes(alpha, beta))
  check(
    !((Model.getNote(unlinked, alpha)->Option.getExn).links->Array.includes(beta)),
    "unlink removes forward link",
  )
  check(!(Model.getBacklinks(unlinked, beta)->Array.includes(alpha)), "unlink removes backlink")

  // Search through the core
  let model = Update.update(model, Msg.SetSearchQuery("alpha"))
  check(model.searchResults == [alpha], "search finds the matching note")
  let model = Update.update(model, Msg.SetSearchQuery(""))
  check(model.searchResults == [], "empty query yields no results")

  // Zoom clamping
  let model = Update.update(model, Msg.ZoomCanvas(100.0))
  check(model.viewport.zoom <= 5.0, "zoom clamped high")
  let model = Update.update(model, Msg.ZoomCanvas(0.00001))
  check(model.viewport.zoom >= 0.1, "zoom clamped low")

  // Move on canvas
  let model = Update.update(model, Msg.MoveNote(alpha, {x: 42.0, y: 7.0}))
  check(
    switch (Model.getNote(model, alpha)->Option.getExn).position {
    | Some(p) => p.x == 42.0 && p.y == 7.0
    | None => false
    },
    "move sets position",
  )

  // Delete is guarded while editing
  let model = Update.update(model, Msg.SelectNote(alpha))
  let editing = Update.update(model, Msg.StartEditingNote(alpha))
  let guarded = Update.update(editing, Msg.DeleteSelectedNotes)
  check(Model.noteCount(guarded) == 2, "delete guarded while editing")

  // Delete works otherwise, and cleans backlinks
  let model = Update.update(model, Msg.StopEditingNote)
  let model = Update.update(model, Msg.LinkNotes(alpha, beta))
  let model = Update.update(model, Msg.DeleteSelectedNotes)
  check(Model.noteCount(model) == 1, "delete removes selected note")
  check(Model.getBacklinks(model, beta) == [], "backlinks cleaned after delete")
  check(model.selection == NoSelection, "selection cleared after delete")
}
