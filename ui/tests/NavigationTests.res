// SPDX-License-Identifier: MPL-2.0
/// Pure unit tests for keyboard-navigation geometry (no DOM, no wasm).

open Types

exception AssertionFailed(string)

let check = (cond: bool, label: string) =>
  if !cond {
    raise(AssertionFailed(label))
  }

let noteAt = (id: noteId, x: float, y: float): note => {
  id,
  title: id,
  content: "",
  position: Some({x, y}),
  size: None,
  createdAt: "",
  modifiedAt: "",
  links: [],
  prototype: None,
  attributes: Js.Dict.empty(),
}

let runAll = () => {
  // Layout:  center at (0,0); right (100,0); up (0,-100); far-right-drift (200,90)
  let center = noteAt("center", 0.0, 0.0)
  let right = noteAt("right", 100.0, 0.0)
  let up = noteAt("up", 0.0, -100.0)
  let drift = noteAt("drift", 200.0, 90.0)
  let notes = [center, right, up, drift]

  check(
    Navigation.nearestInDirection(~notes, ~fromId="center", ~direction=Right) == Some("right"),
    "right picks the straight-ahead note over the drifting one",
  )
  check(
    Navigation.nearestInDirection(~notes, ~fromId="center", ~direction=Up) == Some("up"),
    "up picks the note above",
  )
  check(
    Navigation.nearestInDirection(~notes, ~fromId="center", ~direction=Left) == None,
    "nothing to the left",
  )
  check(
    Navigation.nearestInDirection(~notes, ~fromId="center", ~direction=Down) == Some("drift"),
    "down reaches the only note below (drift, at y=90)",
  )

  // Unknown origin yields nothing.
  check(
    Navigation.nearestInDirection(~notes, ~fromId="missing", ~direction=Right) == None,
    "unknown origin yields none",
  )

  // Nudge geometry.
  let moved = Navigation.nudge(~position={x: 10.0, y: 10.0}, ~direction=Down)
  check(moved.y == 10.0 +. Navigation.nudgeStep && moved.x == 10.0, "nudge down adds to y only")
  let movedLeft = Navigation.nudge(~position={x: 10.0, y: 10.0}, ~direction=Left)
  check(movedLeft.x == 10.0 -. Navigation.nudgeStep, "nudge left subtracts from x")
}
