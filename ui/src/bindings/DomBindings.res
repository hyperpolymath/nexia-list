// SPDX-License-Identifier: MPL-2.0
/// Minimal DOM externals for global keyboard handling.
/// Local bindings instead of a rescript-webapi dependency: only these six
/// operations are needed.

type keyboardEvent

@val @scope("window")
external addKeydownListener: (@as("keydown") _, keyboardEvent => unit) => unit = "addEventListener"

@val @scope("window")
external removeKeydownListener: (@as("keydown") _, keyboardEvent => unit) => unit =
  "removeEventListener"

@get external key: keyboardEvent => string = "key"
@get external ctrlKey: keyboardEvent => bool = "ctrlKey"
@get external metaKey: keyboardEvent => bool = "metaKey"
@get external shiftKey: keyboardEvent => bool = "shiftKey"
@get external altKey: keyboardEvent => bool = "altKey"
@send external preventDefault: keyboardEvent => unit = "preventDefault"

// Tag name of the event target ("INPUT", "TEXTAREA", "BODY", ...), used to
// avoid hijacking arrow keys while the user is typing.
type eventTarget
@get external target: keyboardEvent => eventTarget = "target"
@get external tagName: eventTarget => string = "tagName"
let targetTag = (e: keyboardEvent): string => tagName(target(e))

/// True when focus is in a text field, so global shortcuts should defer.
let isTyping = (e: keyboardEvent): bool =>
  switch targetTag(e) {
  | "INPUT" | "TEXTAREA" => true
  | _ => false
  }

// Document-level mouse tracking for canvas drag / pan (events must be caught
// even when the pointer leaves the note being dragged).
type mouseEvent
@get external mouseClientX: mouseEvent => float = "clientX"
@get external mouseClientY: mouseEvent => float = "clientY"

@val @scope("document")
external addMouseMove: (@as("mousemove") _, mouseEvent => unit) => unit = "addEventListener"
@val @scope("document")
external removeMouseMove: (@as("mousemove") _, mouseEvent => unit) => unit = "removeEventListener"
@val @scope("document")
external addMouseUp: (@as("mouseup") _, mouseEvent => unit) => unit = "addEventListener"
@val @scope("document")
external removeMouseUp: (@as("mouseup") _, mouseEvent => unit) => unit = "removeEventListener"
