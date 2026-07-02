// SPDX-License-Identifier: MPL-2.0
/// Main entry point for Nexia: loads the WASM core, restores the autosaved
/// notebook from IndexedDB, then mounts the app.

module App = {
  @react.component
  let make = (~initialNotebook: Types.notebook) => {
    let (model, setModel) = React.useState(() => {
      ...Model.initial(),
      notebook: initialNotebook,
    })

    let dispatch = (msg: Msg.msg) => {
      setModel(currentModel => Update.update(currentModel, msg))
    }

    React.useEffect0(() => {
      Dispatcher.dispatchRef := dispatch
      None
    })

    // Any notebook change (edits, load, new) refreshes the IndexedDB copy
    // after a debounce, so the working set survives reloads.
    React.useEffect1(() => {
      Persist.scheduleAutosave()
      None
    }, [model.notebook])

    // Keyboard shortcuts. The handler is registered once; guards that need
    // current state (e.g. "don't delete while editing") live in Update.update,
    // which always sees the latest model.
    React.useEffect0(() => {
      let handleKeyDown = (e: DomBindings.keyboardEvent) => {
        let key = DomBindings.key(e)
        let modKey = DomBindings.ctrlKey(e) || DomBindings.metaKey(e)

        switch (modKey, key) {
        | (true, "n") => {
            DomBindings.preventDefault(e)
            dispatch(Msg.CreateNote)
          }
        | (true, "s") => {
            DomBindings.preventDefault(e)
            dispatch(Msg.SaveNotebook)
          }
        | (false, "Escape") => {
            dispatch(Msg.ClearSelection)
            dispatch(Msg.StopEditingNote)
          }
        | (false, "Delete") | (false, "Backspace") => dispatch(Msg.DeleteSelectedNotes)
        | _ => ()
        }
      }

      DomBindings.addKeydownListener(handleKeyDown)
      Some(() => DomBindings.removeKeydownListener(handleKeyDown))
    })

    <View model dispatch />
  }
}

let start = async () => {
  try {
    await WasmStore.init("./wasm/nexia_core_bg.wasm")

    let initialNotebook = switch await Persist.loadAutosave() {
    | Some(json) =>
      switch WasmStore.loadFromJson(json) {
      | Ok(snapshot) => snapshot
      | Error(_) => WasmStore.snapshot() // unreadable autosave: start fresh
      }
    | None => WasmStore.snapshot()
    }

    switch ReactDOM.querySelector("#root") {
    | Some(root) =>
      ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(<App initialNotebook />)
    | None => Js.Console.error("Could not find #root element")
    }
  } catch {
  | e => {
      Js.Console.error2("Nexia failed to start", e->Exn.anyToExnInternal)
      switch ReactDOM.querySelector("#root") {
      | Some(root) =>
        ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(
          <div role="alert" className="error-banner">
            {React.string("Nexia failed to start. Check the browser console for details.")}
          </div>,
        )
      | None => ()
      }
    }
  }
}

start()->ignore
