// SPDX-License-Identifier: AGPL-3.0-or-later
/// View functions - render the UI

open Types
open Model
open Msg

module Sidebar = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let notes = allNotes(model)->Array.toSorted((a, b) =>
      String.localeCompare(a.title, b.title)
    )

    <aside className="sidebar">
      <div className="sidebar-header">
        <h2> {React.string(model.notebook.name)} </h2>
        <button onClick={_ => dispatch(CreateNote)} className="btn-primary">
          {React.string("+ New Note")}
        </button>
      </div>
      <div className="search-box">
        <input
          type_="text"
          placeholder="Search notes..."
          value={model.searchQuery}
          onChange={e => dispatch(SetSearchQuery(ReactEvent.Form.target(e)["value"]))}
        />
        {model.searchQuery != ""
          ? <button onClick={_ => dispatch(ClearSearch)} className="btn-clear">
              {React.string("×")}
            </button>
          : React.null}
      </div>
      <ul className="note-list">
        {(
          model.searchQuery != "" ? model.searchResults : notes->Array.map(n => n.id)
        )
        ->Array.map(id => {
          switch getNote(model, id) {
          | Some(note) =>
            let isSelected = switch model.selection {
            | SingleNote(selectedId) => selectedId == id
            | MultipleNotes(ids) => Array.includes(ids, id)
            | NoSelection => false
            }
            <li
              key={id}
              className={isSelected ? "note-item selected" : "note-item"}
              onClick={_ => dispatch(SelectNote(id))}>
              <span className="note-title">
                {React.string(note.title != "" ? note.title : "Untitled")}
              </span>
              <span className="note-meta">
                {React.string(
                  `${Array.length(note.links)->Int.toString} links`,
                )}
              </span>
            </li>
          | None => React.null
          }
        })
        ->React.array}
      </ul>
      <div className="sidebar-footer">
        <span className="note-count">
          {React.string(`${noteCount(model)->Int.toString} notes`)}
        </span>
      </div>
    </aside>
  }
}

module NoteEditor = {
  @react.component
  let make = (~note: note, ~dispatch: msg => unit) => {
    <div className="note-editor">
      <input
        type_="text"
        className="note-title-input"
        placeholder="Note title"
        value={note.title}
        onChange={e =>
          dispatch(UpdateNoteTitle(note.id, ReactEvent.Form.target(e)["value"]))}
        onBlur={_ => dispatch(StopEditingNote)}
      />
      <textarea
        className="note-content-input"
        placeholder="Start writing..."
        value={note.content}
        onChange={e =>
          dispatch(UpdateNoteContent(note.id, ReactEvent.Form.target(e)["value"]))}
      />
      <div className="note-metadata">
        <span> {React.string(`Created: ${note.createdAt}`)} </span>
        <span> {React.string(`Modified: ${note.modifiedAt}`)} </span>
      </div>
      {note.links->Array.length > 0
        ? <div className="note-links">
            <h4> {React.string("Links")} </h4>
            <ul>
              {note.links
              ->Array.map(linkId =>
                <li key={linkId}>
                  <button onClick={_ => dispatch(SelectNote(linkId))}>
                    {React.string(linkId)}
                  </button>
                  <button
                    onClick={_ => dispatch(UnlinkNotes(note.id, linkId))} className="btn-remove">
                    {React.string("×")}
                  </button>
                </li>
              )
              ->React.array}
            </ul>
          </div>
        : React.null}
    </div>
  }
}

module ListView = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let selectedNote = switch model.selection {
    | SingleNote(id) => getNote(model, id)
    | _ => None
    }

    <div className="list-view">
      {switch selectedNote {
      | Some(note) => <NoteEditor note dispatch />
      | None =>
        <div className="empty-state">
          <p> {React.string("Select a note or create a new one")} </p>
          <button onClick={_ => dispatch(CreateNote)} className="btn-primary">
            {React.string("Create Note")}
          </button>
        </div>
      }}
    </div>
  }
}

module CanvasView = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let notes = allNotes(model)->Array.filter(n => n.position->Option.isSome)

    <div className="canvas-view">
      <div
        className="canvas"
        style={ReactDOM.Style.make(
          ~transform=`translate(${model.viewport.offsetX->Float.toString}px, ${model.viewport.offsetY->Float.toString}px) scale(${model.viewport.zoom->Float.toString})`,
          (),
        )}
        onDoubleClick={e => {
          let rect = ReactEvent.Mouse.currentTarget(e)["getBoundingClientRect"]()
          let x = (ReactEvent.Mouse.clientX(e)->Int.toFloat -. rect["left"]) /. model.viewport.zoom
          let y = (ReactEvent.Mouse.clientY(e)->Int.toFloat -. rect["top"]) /. model.viewport.zoom
          dispatch(CreateNoteAt({x, y}))
        }}>
        {notes
        ->Array.map(note => {
          let pos = note.position->Option.getExn
          let isSelected = switch model.selection {
          | SingleNote(id) => id == note.id
          | MultipleNotes(ids) => Array.includes(ids, note.id)
          | NoSelection => false
          }
          <div
            key={note.id}
            className={isSelected ? "canvas-note selected" : "canvas-note"}
            style={ReactDOM.Style.make(
              ~left=`${pos.x->Float.toString}px`,
              ~top=`${pos.y->Float.toString}px`,
              (),
            )}
            onClick={_ => dispatch(SelectNote(note.id))}
            onDoubleClick={e => {
              ReactEvent.Mouse.stopPropagation(e)
              dispatch(StartEditingNote(note.id))
            }}>
            <div className="canvas-note-title">
              {React.string(note.title != "" ? note.title : "Untitled")}
            </div>
            {note.content != ""
              ? <div className="canvas-note-preview">
                  {React.string(
                    String.slice(note.content, ~start=0, ~end=100) ++
                    (String.length(note.content) > 100 ? "..." : ""),
                  )}
                </div>
              : React.null}
          </div>
        })
        ->React.array}
      </div>
      <div className="canvas-controls">
        <button onClick={_ => dispatch(ZoomCanvas(1.2))}> {React.string("+")} </button>
        <button onClick={_ => dispatch(ZoomCanvas(0.8))}> {React.string("-")} </button>
        <button onClick={_ => dispatch(ResetViewport)}> {React.string("Reset")} </button>
      </div>
    </div>
  }
}

module Toolbar = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    <div className="toolbar">
      <div className="toolbar-left">
        <button
          onClick={_ => dispatch(ToggleSidebar)}
          className={model.sidebarOpen ? "btn-active" : ""}>
          {React.string("Sidebar")}
        </button>
      </div>
      <div className="toolbar-center">
        <button
          onClick={_ => dispatch(SetViewMode(ListView))}
          className={model.viewMode == ListView ? "btn-active" : ""}>
          {React.string("List")}
        </button>
        <button
          onClick={_ => dispatch(SetViewMode(CanvasView))}
          className={model.viewMode == CanvasView ? "btn-active" : ""}>
          {React.string("Canvas")}
        </button>
      </div>
      <div className="toolbar-right">
        {model.dirty
          ? <span className="dirty-indicator"> {React.string("Unsaved")} </span>
          : React.null}
        <button onClick={_ => dispatch(NewNotebook)}> {React.string("New")} </button>
        <button onClick={_ => dispatch(SaveNotebook)}> {React.string("Save")} </button>
      </div>
    </div>
  }
}

module ErrorBanner = {
  @react.component
  let make = (~error: option<string>, ~dispatch: msg => unit) => {
    switch error {
    | Some(message) =>
      <div className="error-banner">
        <span> {React.string(message)} </span>
        <button onClick={_ => dispatch(ClearError)}> {React.string("×")} </button>
      </div>
    | None => React.null
    }
  }
}

/// Main view function
@react.component
let make = (~model: model, ~dispatch: msg => unit) => {
  <div className="app">
    <ErrorBanner error={model.error} dispatch />
    <Toolbar model dispatch />
    <div className="main-content">
      {model.sidebarOpen ? <Sidebar model dispatch /> : React.null}
      <main className="content-area">
        {switch model.viewMode {
        | ListView => <ListView model dispatch />
        | CanvasView => <CanvasView model dispatch />
        | GraphView => <div> {React.string("Graph view coming soon")} </div>
        }}
      </main>
    </div>
  </div>
}
