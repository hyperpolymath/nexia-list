// SPDX-License-Identifier: AGPL-3.0-or-later
/// Core types for Nexia UI - mirrors Rust core types

/// Unique identifier for a note
type noteId = string

/// 2D position on the spatial canvas
type point2D = {
  x: float,
  y: float,
}

/// A single note in the knowledge graph
type note = {
  id: noteId,
  title: string,
  content: string,
  position: option<point2D>,
  size: option<(float, float)>,
  createdAt: string, // ISO 8601 datetime
  modifiedAt: string,
  links: array<noteId>,
  prototype: option<noteId>,
  attributes: Js.Dict.t<Js.Json.t>,
}

/// Notebook containing all notes
type notebook = {
  notes: Js.Dict.t<note>,
  backlinks: Js.Dict.t<array<noteId>>,
  name: string,
  createdAt: string,
  modifiedAt: string,
}

/// View mode for the application
type viewMode =
  | ListView
  | CanvasView
  | GraphView

/// Selection state
type selection =
  | NoSelection
  | SingleNote(noteId)
  | MultipleNotes(array<noteId>)

/// Canvas viewport state
type viewport = {
  offsetX: float,
  offsetY: float,
  zoom: float,
}

/// Helper functions for creating types
module Note = {
  let make = (~title: string): note => {
    let now = Js.Date.toISOString(Js.Date.make())
    {
      id: Js.Math.random()->Float.toString,
      title,
      content: "",
      position: None,
      size: None,
      createdAt: now,
      modifiedAt: now,
      links: [],
      prototype: None,
      attributes: Js.Dict.empty(),
    }
  }

  let withPosition = (note: note, x: float, y: float): note => {
    ...note,
    position: Some({x, y}),
  }
}

module Point2D = {
  let make = (x: float, y: float): point2D => {x, y}
  let origin = (): point2D => {x: 0.0, y: 0.0}
}

module Viewport = {
  let initial = (): viewport => {
    offsetX: 0.0,
    offsetY: 0.0,
    zoom: 1.0,
  }
}
