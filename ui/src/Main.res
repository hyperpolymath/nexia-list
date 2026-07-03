// SPDX-License-Identifier: MPL-2.0
/// Main entry point for Nexia-List: loads the WASM core, restores the autosaved
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
      // Load any agents restored from the autosaved notebook.
      dispatch(Msg.RefreshAgents)
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
        let typing = DomBindings.isTyping(e)

        // Arrow keys drive keyboard-only canvas navigation. Alt+arrow nudges
        // the selected note; a plain arrow moves the selection. Suppressed
        // while typing so text fields keep their caret movement.
        let arrow = switch key {
        | "ArrowUp" => Some(Types.Up)
        | "ArrowDown" => Some(Types.Down)
        | "ArrowLeft" => Some(Types.Left)
        | "ArrowRight" => Some(Types.Right)
        | _ => None
        }

        switch (modKey, key, arrow, typing) {
        | (true, "n", _, _) => {
            DomBindings.preventDefault(e)
            dispatch(Msg.CreateNote)
          }
        | (true, "s", _, _) => {
            DomBindings.preventDefault(e)
            dispatch(Msg.SaveNotebook)
          }
        | (_, _, Some(direction), false) => {
            DomBindings.preventDefault(e)
            if DomBindings.altKey(e) {
              dispatch(Msg.NudgeSelectedNote(direction))
            } else {
              dispatch(Msg.NavigateCanvas(direction))
            }
          }
        | (false, "Escape", _, _) => {
            dispatch(Msg.ClearSelection)
            dispatch(Msg.StopEditingNote)
          }
        | (false, "Delete", _, false) | (false, "Backspace", _, false) =>
          dispatch(Msg.DeleteSelectedNotes)
        | _ => ()
        }
      }

      DomBindings.addKeydownListener(handleKeyDown)
      Some(() => DomBindings.removeKeydownListener(handleKeyDown))
    })

    <View model dispatch />
  }
}

// Register the offline service worker (best-effort; never blocks startup).
%%raw(`
if ("serviceWorker" in navigator) {
  globalThis.addEventListener("load", () => {
    navigator.serviceWorker.register("./service-worker.js").catch(() => {});
  });
}
`)

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
      Js.Console.error2("Nexia-List failed to start", e->Exn.anyToExnInternal)
      switch ReactDOM.querySelector("#root") {
      | Some(root) =>
        ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(
          <div role="alert" className="error-banner">
            {React.string("Nexia-List failed to start. Check the browser console for details.")}
          </div>,
        )
      | None => ()
      }
    }
  }
}

start()->ignore
