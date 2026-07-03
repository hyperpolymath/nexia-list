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
