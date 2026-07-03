// SPDX-License-Identifier: MPL-2.0
/// Import/export effects bridging WasmStore to the browser file APIs.
/// These are async and complete after update() returns, so results come back
/// through Dispatcher (NotebookLoaded / SetError).

open Msg

@module("./vault.js")
external exportMarkdownVault: array<WasmStore.markdownFile> => promise<string> =
  "exportMarkdownVault"
@module("./vault.js") external downloadOpml: (string, string) => unit = "downloadOpml"
@module("./vault.js")
external pickMarkdownVault: unit => promise<array<WasmStore.markdownFile>> = "pickMarkdownVault"

let exportMarkdown = () =>
  switch WasmStore.exportMarkdown() {
  | Ok(files) => exportMarkdownVault(files)->Promise.thenResolve(_ => ())->ignore
  | Error(message) => Dispatcher.dispatch(SetError(message))
  }

let exportOpml = (name: string) =>
  switch WasmStore.exportOpml() {
  | Ok(opml) => downloadOpml(name, opml)
  | Error(message) => Dispatcher.dispatch(SetError(message))
  }

let importVault = () =>
  pickMarkdownVault()
  ->Promise.thenResolve(files =>
    if Array.length(files) > 0 {
      switch WasmStore.importVault(files) {
      | Ok(snapshot) => Dispatcher.dispatch(NotebookLoaded(snapshot))
      | Error(message) => Dispatcher.dispatch(SetError(message))
      }
    }
  )
  ->ignore
