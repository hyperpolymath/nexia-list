// SPDX-License-Identifier: MPL-2.0
/// Late-bound dispatch for effects (e.g. the file picker) that complete
/// after update() has returned. Main.res registers the live dispatcher on
/// mount.

let dispatchRef: ref<Msg.msg => unit> = ref(_ => ())

let dispatch = (msg: Msg.msg) => dispatchRef.contents(msg)
