// SPDX-License-Identifier: AGPL-3.0-or-later
/// Main entry point for Nexia

module App = {
  @react.component
  let make = () => {
    let (model, setModel) = React.useState(() => Model.initial())

    let dispatch = (msg: Msg.msg) => {
      setModel(currentModel => Update.update(currentModel, msg))
    }

    // Keyboard shortcuts
    React.useEffect0(() => {
      let handleKeyDown = (e: Dom.keyboardEvent) => {
        let key = Webapi.Dom.KeyboardEvent.key(e)
        let ctrlKey = Webapi.Dom.KeyboardEvent.ctrlKey(e)
        let metaKey = Webapi.Dom.KeyboardEvent.metaKey(e)
        let modKey = ctrlKey || metaKey

        switch (modKey, key) {
        | (true, "n") => {
            Webapi.Dom.KeyboardEvent.preventDefault(e)
            dispatch(Msg.CreateNote)
          }
        | (true, "s") => {
            Webapi.Dom.KeyboardEvent.preventDefault(e)
            dispatch(Msg.SaveNotebook)
          }
        | (true, "f") => {
            Webapi.Dom.KeyboardEvent.preventDefault(e)
            // Focus search - would need a ref
          }
        | (false, "Escape") => {
            dispatch(Msg.ClearSelection)
            dispatch(Msg.StopEditingNote)
          }
        | (false, "Delete") | (false, "Backspace") =>
          // Only delete if not editing
          switch model.editingNote {
          | None => dispatch(Msg.DeleteSelectedNotes)
          | Some(_) => ()
          }
        | _ => ()
        }
      }

      let handler = %raw(`
        function(e) {
          handleKeyDown(e);
        }
      `)

      Webapi.Dom.window->Webapi.Dom.Window.addEventListener("keydown", handler)

      Some(
        () => {
          Webapi.Dom.window->Webapi.Dom.Window.removeEventListener("keydown", handler)
        },
      )
    })

    <View model dispatch />
  }
}

// Mount the app
switch ReactDOM.querySelector("#root") {
| Some(root) => ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(<App />)
| None => Js.Console.error("Could not find #root element")
}
