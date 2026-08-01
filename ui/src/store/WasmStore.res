// SPDX-License-Identifier: MPL-2.0
/// Bindings to the WASM-compiled Rust core plus the current-notebook handle.
///
/// This module is the seam between the TEA UI and the engine: Update.res
/// mutates through it and patches its read model with the returned values.
/// A GossamerStore implementing the same operations over the desktop bridge
/// can replace it in a webview shell without UI changes.

open Types

type t

type delta = {
  changed: array<note>,
  removed: array<noteId>,
  backlinks: Dict.t<array<noteId>>,
}

@module("../../../web/wasm/nexia_core.js")
external initWasm: string => promise<unit> = "default"

@module("../../../web/wasm/nexia_core.js")
external lambdadeltaEvalRaw: string => string = "lambdadeltaEval"

@module("../../../web/wasm/nexia_core.js") @new
external makeNotebook: string => t = "WasmNotebook"

@module("../../../web/wasm/nexia_core.js") @scope("WasmNotebook")
external fromJsonRaw: string => t = "from_json"

@send external evalLambdadeltaRaw: (t, string) => string = "evalLambdadelta"
@send external evalFormulaRaw: (t, noteId, string) => string = "evalFormula"
@send external nameRaw: t => string = "name"
@send external setNameRaw: (t, string) => unit = "set_name"
@send external lenRaw: t => int = "len"
@send external isEmptyRaw: t => bool = "is_empty"
@send external createNoteRaw: (t, string) => note = "create_note"
@send external createNoteAtRaw: (t, string, float, float) => note = "create_note_at"
@send external getNoteRaw: (t, noteId) => option<note> = "get_note"
@send external updateTitleRaw: (t, noteId, string) => note = "update_title"
@send external updateContentRaw: (t, noteId, string) => delta = "update_content"
@send external moveNoteRaw: (t, noteId, float, float) => note = "move_note"
@send external resizeNoteRaw: (t, noteId, float, float) => note = "resize_note"
@send external setAttributeRaw: (t, noteId, string, string) => note = "set_attribute"
@send external deleteNoteRaw: (t, noteId) => delta = "delete_note"
@send external linkRaw: (t, noteId, noteId) => delta = "link"
@send external unlinkRaw: (t, noteId, noteId) => delta = "unlink"
@send external backlinksRaw: (t, noteId) => array<noteId> = "backlinks"
@send external searchRaw: (t, string) => array<noteId> = "search"
@send external snapshotRaw: t => notebook = "snapshot"
@send external toJsonRaw: t => string = "to_json"

type markdownFile = {name: string, content: string}
@send external exportMarkdownRaw: t => array<markdownFile> = "export_markdown"
@send external exportOpmlRaw: t => string = "export_opml"
@send external importVaultRaw: (t, array<markdownFile>) => notebook = "import_markdown_vault"

@send external agentsRaw: t => array<agent> = "agents"
@send external addAgentRaw: (t, string, string) => agent = "add_agent"
@send external removeAgentRaw: (t, agentId) => bool = "remove_agent"
@send external runAgentRaw: (t, agentId) => array<noteId> = "run_agent"
@send external runQueryRaw: (t, string) => array<noteId> = "run_query"

let current: ref<option<t>> = ref(None)

exception StoreNotInitialized

let instance = (): t =>
  switch current.contents {
  | Some(store) => store
  | None => raise(StoreNotInitialized)
  }

/// Load the wasm module and start with an empty notebook.
let init = async (wasmUrl: string) => {
  await initWasm(wasmUrl)
  current := Some(makeNotebook("Untitled Notebook"))
}

/// Replace the current notebook with a fresh one; returns its snapshot.
let reset = (name: string): notebook => {
  let nb = makeNotebook(name)
  current := Some(nb)
  snapshotRaw(nb)
}

let errorMessage = (e: exn, fallback: string): string =>
  switch e {
  | Exn.Error(jsError) => jsError->Exn.message->Option.getOr(fallback)
  | _ => fallback
  }

/// Replace the current notebook from on-disk JSON; returns its snapshot.
let loadFromJson = (json: string): result<notebook, string> =>
  try {
    let nb = fromJsonRaw(json)
    current := Some(nb)
    Ok(snapshotRaw(nb))
  } catch {
  | e => Error(errorMessage(e, "Could not load notebook"))
  }

let snapshot = (): notebook => snapshotRaw(instance())

let toJson = (): result<string, string> =>
  try Ok(toJsonRaw(instance())) catch {
  | e => Error(errorMessage(e, "Could not serialize notebook"))
  }

let tryNote = (operation: unit => note): result<note, string> =>
  try Ok(operation()) catch {
  | e => Error(errorMessage(e, "Note operation failed"))
  }

let tryDelta = (operation: unit => delta): result<delta, string> =>
  try Ok(operation()) catch {
  | e => Error(errorMessage(e, "Link operation failed"))
  }

let tryString = (operation: unit => string, fallback: string): result<string, string> =>
  try Ok(operation()) catch {
  | e => Error(errorMessage(e, fallback))
  }

/// Evaluate pure LambdaDelta without notebook host bindings.
let lambdadeltaEval = (source: string): result<string, string> =>
  tryString(() => lambdadeltaEvalRaw(source), "LambdaDelta evaluation failed")

/// Evaluate LambdaDelta with the current notebook host. Programs may mutate
/// the core notebook; callers that keep a read model should refresh snapshot().
let evalLambdadelta = (source: string): result<string, string> =>
  tryString(() => evalLambdadeltaRaw(instance(), source), "Notebook LambdaDelta evaluation failed")

/// Evaluate a read-only formula with `self` bound to the selected note.
let evalFormula = (id: noteId, source: string): result<string, string> =>
  tryString(() => evalFormulaRaw(instance(), id, source), "LambdaDelta formula evaluation failed")

let name = (): string => nameRaw(instance())
let setName = (name: string): unit => setNameRaw(instance(), name)
let len = (): int => lenRaw(instance())
let isEmpty = (): bool => isEmptyRaw(instance())

let createNote = (title: string) => tryNote(() => createNoteRaw(instance(), title))
let createNoteAt = (title: string, x: float, y: float) =>
  tryNote(() => createNoteAtRaw(instance(), title, x, y))
let getNote = (id: noteId): result<option<note>, string> =>
  try Ok(getNoteRaw(instance(), id)) catch {
  | e => Error(errorMessage(e, "Could not read note"))
  }
let updateTitle = (id: noteId, title: string) => tryNote(() => updateTitleRaw(instance(), id, title))
let updateContent = (id: noteId, content: string) =>
  tryDelta(() => updateContentRaw(instance(), id, content))
let moveNote = (id: noteId, x: float, y: float) => tryNote(() => moveNoteRaw(instance(), id, x, y))
let resizeNote = (id: noteId, width: float, height: float) =>
  tryNote(() => resizeNoteRaw(instance(), id, width, height))
let setAttribute = (id: noteId, key: string, jsonValue: string) =>
  tryNote(() => setAttributeRaw(instance(), id, key, jsonValue))

let deleteNote = (id: noteId) => tryDelta(() => deleteNoteRaw(instance(), id))
let link = (fromId: noteId, toId: noteId) => tryDelta(() => linkRaw(instance(), fromId, toId))
let unlink = (fromId: noteId, toId: noteId) => tryDelta(() => unlinkRaw(instance(), fromId, toId))

let backlinks = (id: noteId): result<array<noteId>, string> =>
  try Ok(backlinksRaw(instance(), id)) catch {
  | e => Error(errorMessage(e, "Could not read backlinks"))
  }

let search = (query: string): array<noteId> => searchRaw(instance(), query)

let exportMarkdown = (): result<array<markdownFile>, string> =>
  try Ok(exportMarkdownRaw(instance())) catch {
  | e => Error(errorMessage(e, "Could not export Markdown"))
  }

let exportOpml = (): result<string, string> =>
  try Ok(exportOpmlRaw(instance())) catch {
  | e => Error(errorMessage(e, "Could not export OPML"))
  }

/// Replace the current notebook by importing a Markdown vault.
let importVault = (files: array<markdownFile>): result<notebook, string> =>
  try Ok(importVaultRaw(instance(), files)) catch {
  | e => Error(errorMessage(e, "Could not import vault"))
  }

let agents = (): array<agent> =>
  switch current.contents {
  | Some(nb) => agentsRaw(nb)
  | None => []
  }

let addAgent = (name: string, query: string): result<agent, string> =>
  try Ok(addAgentRaw(instance(), name, query)) catch {
  | e => Error(errorMessage(e, "Could not create agent"))
  }

let removeAgent = (id: agentId): bool =>
  switch current.contents {
  | Some(nb) =>
    try removeAgentRaw(nb, id) catch {
    | _ => false
    }
  | None => false
  }

let runAgent = (id: agentId): array<noteId> =>
  switch current.contents {
  | Some(nb) =>
    try runAgentRaw(nb, id) catch {
    | _ => []
    }
  | None => []
  }

let runQuery = (query: string): array<noteId> =>
  switch current.contents {
  | Some(nb) => runQueryRaw(nb, query)
  | None => []
  }
