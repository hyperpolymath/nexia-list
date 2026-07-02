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
@send external preventDefault: keyboardEvent => unit = "preventDefault"
