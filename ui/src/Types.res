// SPDX-License-Identifier: MPL-2.0
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

// Note construction lives in the Rust core (WasmStore.createNote) — the UI
// never fabricates ids or timestamps, which is what kept these types from
// drifting apart before.

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
