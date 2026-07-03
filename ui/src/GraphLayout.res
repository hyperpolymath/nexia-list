// SPDX-License-Identifier: MPL-2.0
/// Deterministic layout for the graph view. A circular arrangement keeps the
/// result static (no animation — friendly to prefers-reduced-motion) and
/// pure, so it is unit-testable without a DOM. Coordinates are always finite
/// and inside the given box.

open Types

type nodePos = {
  id: noteId,
  title: string,
  x: float,
  y: float,
}

/// Place notes evenly on a circle inscribed in width×height.
let circular = (~notes: array<note>, ~width: float, ~height: float): array<nodePos> => {
  let n = Array.length(notes)
  if n == 0 {
    []
  } else {
    let cx = width /. 2.0
    let cy = height /. 2.0
    let radius = Js.Math.min_float(width, height) *. 0.38
    notes->Array.mapWithIndex((note, i) => {
      // Start at the top (−90°) and go clockwise.
      let angle =
        -.Js.Math._PI /. 2.0 +. 2.0 *. Js.Math._PI *. Int.toFloat(i) /. Int.toFloat(n)
      {
        id: note.id,
        title: note.title == "" ? "Untitled" : note.title,
        x: cx +. radius *. Js.Math.cos(angle),
        y: cy +. radius *. Js.Math.sin(angle),
      }
    })
  }
}

/// Index positions by note id for edge lookup.
let byId = (positions: array<nodePos>): Js.Dict.t<nodePos> => {
  let dict = Js.Dict.empty()
  positions->Array.forEach(p => Js.Dict.set(dict, p.id, p))
  dict
}
