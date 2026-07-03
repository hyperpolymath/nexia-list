// SPDX-License-Identifier: MPL-2.0
/// Pure unit tests for the graph layout (no DOM).

open Types

exception AssertionFailed(string)

let check = (cond: bool, label: string) =>
  if !cond {
    raise(AssertionFailed(label))
  }

let noteAt = (id: noteId): note => {
  id,
  title: id,
  content: "",
  position: None,
  size: None,
  createdAt: "",
  modifiedAt: "",
  links: [],
  prototype: None,
  attributes: Js.Dict.empty(),
}

let isFinite = (f: float): bool => Js.Float.isFinite(f)

let runAll = () => {
  check(GraphLayout.circular(~notes=[], ~width=800.0, ~height=600.0) == [], "empty in, empty out")

  let notes = ["a", "b", "c", "d", "e"]->Array.map(noteAt)
  let width = 800.0
  let height = 600.0
  let positions = GraphLayout.circular(~notes, ~width, ~height)

  check(Array.length(positions) == 5, "one position per note")
  positions->Array.forEach(p => {
    check(isFinite(p.x) && isFinite(p.y), "coordinates are finite")
    check(p.x >= 0.0 && p.x <= width, "x within box")
    check(p.y >= 0.0 && p.y <= height, "y within box")
  })

  // Every id is indexable.
  let dict = GraphLayout.byId(positions)
  notes->Array.forEach(n => check(Js.Dict.get(dict, n.id)->Option.isSome, "id present in index"))

  // Deterministic: same input → same output.
  let again = GraphLayout.circular(~notes, ~width, ~height)
  check(again == positions, "layout is deterministic")
}
