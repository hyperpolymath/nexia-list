// SPDX-License-Identifier: MPL-2.0
/// Persistence: debounced autosave to IndexedDB plus explicit file
/// save (download) and load (picker). The autosave key always mirrors the
/// current working notebook so it survives reloads.

@module("./idb.js") external idbSet: (string, string) => promise<unit> = "idbSet"
@module("./idb.js") external idbGet: string => promise<Js.Nullable.t<string>> = "idbGet"
@module("./fileio.js") external downloadText: (string, string) => unit = "downloadText"
@module("./fileio.js")
external openTextFile: unit => promise<Js.Nullable.t<string>> = "openTextFile"

let autosaveKey = "nexia.autosave"
let autosaveDelayMs = 800

let pending: ref<option<timeoutId>> = ref(None)

let scheduleAutosave = () => {
  switch pending.contents {
  | Some(id) => clearTimeout(id)
  | None => ()
  }
  pending :=
    Some(setTimeout(() => {
        pending := None
        switch WasmStore.toJson() {
        | Ok(json) => idbSet(autosaveKey, json)->ignore
        | Error(_) => ()
        }
      }, autosaveDelayMs))
}

let loadAutosave = async (): option<string> => {
  switch await idbGet(autosaveKey) {
  | value => value->Js.Nullable.toOption
  | exception _ => None
  }
}

let saveToFile = (name: string, json: string) => {
  let safeName = name == "" ? "notebook" : name
  downloadText(`${safeName}.nexia.json`, json)
}

let openFile = async (): option<string> => {
  switch await openTextFile() {
  | value => value->Js.Nullable.toOption
  | exception _ => None
  }
}
