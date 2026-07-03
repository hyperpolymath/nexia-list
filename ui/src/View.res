// SPDX-License-Identifier: MPL-2.0
/// View functions - render the UI

open Types
open Model
open Msg

// Enter / Space activate an element exposed as role="button".
let onActivateKey = (handler: unit => unit, e: ReactEvent.Keyboard.t) =>
  switch ReactEvent.Keyboard.key(e) {
  | "Enter" | " " =>
    ReactEvent.Keyboard.preventDefault(e)
    handler()
  | _ => ()
  }

module AgentsPanel = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let (name, setName) = React.useState(() => "")
    let (query, setQuery) = React.useState(() => "")

    let submit = _ => {
      let n = String.trim(name)
      let q = String.trim(query)
      if n != "" && q != "" {
        dispatch(CreateAgent(n, q))
        setName(_ => "")
        setQuery(_ => "")
      }
    }

    <section className="agents-panel" ariaLabel="Agents">
      <h3> {React.string("Agents")} </h3>
      <ul className="agent-list">
        {model.agents
        ->Array.map(agent => {
          let isActive = model.activeAgent == Some(agent.id)
          <li key={agent.id} className={isActive ? "agent-item active" : "agent-item"}>
            <button
              type_="button"
              className="agent-name"
              ariaPressed={isActive ? #"true" : #"false"}
              title={agent.query}
              onClick={_ =>
                isActive ? dispatch(ClearAgent) : dispatch(RunAgent(agent.id))}>
              {React.string(agent.name)}
            </button>
            <button
              type_="button"
              className="btn-remove"
              ariaLabel={`Delete agent ${agent.name}`}
              onClick={_ => dispatch(DeleteAgent(agent.id))}>
              {React.string("×")}
            </button>
          </li>
        })
        ->React.array}
      </ul>
      <div className="agent-form">
        <input
          type_="text"
          placeholder="Agent name"
          ariaLabel="Agent name"
          value={name}
          onChange={e => setName(_ => ReactEvent.Form.target(e)["value"])}
        />
        <input
          type_="text"
          placeholder="Query, e.g. attr:status=todo"
          ariaLabel="Agent query"
          value={query}
          onChange={e => setQuery(_ => ReactEvent.Form.target(e)["value"])}
        />
        <button type_="button" className="btn-primary" onClick={submit}>
          {React.string("Add agent")}
        </button>
      </div>
    </section>
  }
}

module Sidebar = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let notes = allNotes(model)->Array.toSorted((a, b) =>
      String.localeCompare(a.title, b.title)
    )
    // What the list shows: search results, else an active agent's collection,
    // else every note.
    let listedIds = if model.searchQuery != "" {
      model.searchResults
    } else if model.activeAgent->Option.isSome {
      model.agentResults
    } else {
      notes->Array.map(n => n.id)
    }

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
          ariaLabel="Search notes"
          value={model.searchQuery}
          onChange={e => dispatch(SetSearchQuery(ReactEvent.Form.target(e)["value"]))}
        />
        {model.searchQuery != ""
          ? <button
              ariaLabel="Clear search" onClick={_ => dispatch(ClearSearch)} className="btn-clear">
              {React.string("×")}
            </button>
          : React.null}
      </div>
      <ul className="note-list" ariaLabel="Notes">
        {listedIds
        ->Array.map(id => {
          switch getNote(model, id) {
          | Some(note) =>
            let isSelected = switch model.selection {
            | SingleNote(selectedId) => selectedId == id
            | MultipleNotes(ids) => Array.includes(ids, id)
            | NoSelection => false
            }
            let label = note.title != "" ? note.title : "Untitled"
            <li key={id}>
              <button
                type_="button"
                ariaPressed={(isSelected) ? #"true" : #"false"}
                ariaLabel={label}
                className={isSelected ? "note-item selected" : "note-item"}
                onClick={_ => dispatch(SelectNote(id))}>
                <span className="note-title"> {React.string(label)} </span>
                <span className="note-meta">
                  {React.string(
                    `${Array.length(note.links)->Int.toString} links`,
                  )}
                </span>
              </button>
            </li>
          | None => React.null
          }
        })
        ->React.array}
      </ul>
      <AgentsPanel model dispatch />
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
          let label = note.title != "" ? note.title : "Untitled"
          <div
            key={note.id}
            role="button"
            tabIndex={0}
            ariaPressed={(isSelected) ? #"true" : #"false"}
            ariaLabel={label}
            className={isSelected ? "canvas-note selected" : "canvas-note"}
            style={ReactDOM.Style.make(
              ~left=`${pos.x->Float.toString}px`,
              ~top=`${pos.y->Float.toString}px`,
              (),
            )}
            onClick={_ => dispatch(SelectNote(note.id))}
            onKeyDown={e => onActivateKey(() => dispatch(StartEditingNote(note.id)), e)}
            onDoubleClick={e => {
              ReactEvent.Mouse.stopPropagation(e)
              dispatch(StartEditingNote(note.id))
            }}>
            <div className="canvas-note-title"> {React.string(label)} </div>
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
        <button ariaLabel="Zoom in" onClick={_ => dispatch(ZoomCanvas(1.2))}>
          {React.string("+")}
        </button>
        <button ariaLabel="Zoom out" onClick={_ => dispatch(ZoomCanvas(0.8))}>
          {React.string("-")}
        </button>
        <button ariaLabel="Reset view" onClick={_ => dispatch(ResetViewport)}>
          {React.string("Reset")}
        </button>
      </div>
    </div>
  }
}

module GraphView = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    let notes = allNotes(model)
    let width = 800.0
    let height = 600.0
    let positions = GraphLayout.circular(~notes, ~width, ~height)
    let posById = GraphLayout.byId(positions)

    <div className="graph-view">
      {Array.length(positions) == 0
        ? <div className="empty-state">
            <p> {React.string("No notes yet — create some to see the graph.")} </p>
          </div>
        : <svg
            className="graph-svg"
            viewBox={`0 0 ${width->Float.toString} ${height->Float.toString}`}
            role="img"
            ariaLabel="Note graph">
            // Edges first so nodes draw on top.
            {notes
            ->Array.flatMap(note =>
              switch Js.Dict.get(posById, note.id) {
              | None => []
              | Some(from) =>
                note.links->Array.filterMap(target =>
                  switch Js.Dict.get(posById, target) {
                  | Some(to_) =>
                    Some(
                      <line
                        key={`${note.id}-${target}`}
                        x1={from.x->Float.toString}
                        y1={from.y->Float.toString}
                        x2={to_.x->Float.toString}
                        y2={to_.y->Float.toString}
                        className="graph-edge"
                      />,
                    )
                  | None => None
                  }
                )
              }
            )
            ->React.array}
            {positions
            ->Array.map(p => {
              let isSelected = switch model.selection {
              | SingleNote(id) => id == p.id
              | MultipleNotes(ids) => Array.includes(ids, p.id)
              | NoSelection => false
              }
              <g
                key={p.id}
                className={isSelected ? "graph-node selected" : "graph-node"}
                role="button"
                tabIndex={0}
                ariaLabel={p.title}
                onClick={_ => dispatch(SelectNote(p.id))}
                onKeyDown={e => onActivateKey(() => dispatch(SelectNote(p.id)), e)}>
                <circle cx={p.x->Float.toString} cy={p.y->Float.toString} r="10" />
                <text x={p.x->Float.toString} y={(p.y -. 16.0)->Float.toString} textAnchor="middle">
                  {React.string(p.title)}
                </text>
              </g>
            })
            ->React.array}
          </svg>}
    </div>
  }
}

module Toolbar = {
  @react.component
  let make = (~model: model, ~dispatch: msg => unit) => {
    <div className="toolbar" role="toolbar" ariaLabel="Main toolbar">
      <div className="toolbar-left">
        <button
          onClick={_ => dispatch(ToggleSidebar)}
          ariaPressed={(model.sidebarOpen) ? #"true" : #"false"}
          className={model.sidebarOpen ? "btn-active" : ""}>
          {React.string("Sidebar")}
        </button>
      </div>
      <div className="toolbar-center">
        <button
          onClick={_ => dispatch(SetViewMode(ListView))}
          ariaPressed={(model.viewMode == ListView) ? #"true" : #"false"}
          className={model.viewMode == ListView ? "btn-active" : ""}>
          {React.string("List")}
        </button>
        <button
          onClick={_ => dispatch(SetViewMode(CanvasView))}
          ariaPressed={(model.viewMode == CanvasView) ? #"true" : #"false"}
          className={model.viewMode == CanvasView ? "btn-active" : ""}>
          {React.string("Canvas")}
        </button>
        <button
          onClick={_ => dispatch(SetViewMode(GraphView))}
          ariaPressed={(model.viewMode == GraphView) ? #"true" : #"false"}
          className={model.viewMode == GraphView ? "btn-active" : ""}>
          {React.string("Graph")}
        </button>
      </div>
      <div className="toolbar-right">
        {model.dirty
          ? <span className="dirty-indicator"> {React.string("Unsaved")} </span>
          : React.null}
        <button onClick={_ => dispatch(NewNotebook)}> {React.string("New")} </button>
        <button onClick={_ => dispatch(LoadNotebook(""))}> {React.string("Load")} </button>
        <button onClick={_ => dispatch(SaveNotebook)}> {React.string("Save")} </button>
        <button onClick={_ => dispatch(ImportVault)} title="Import a Markdown vault">
          {React.string("Import")}
        </button>
        <button onClick={_ => dispatch(ExportMarkdown)} title="Export notes as Markdown">
          {React.string("Export MD")}
        </button>
        <button onClick={_ => dispatch(ExportOpml)} title="Export outline as OPML">
          {React.string("Export OPML")}
        </button>
      </div>
    </div>
  }
}

module ErrorBanner = {
  @react.component
  let make = (~error: option<string>, ~dispatch: msg => unit) => {
    switch error {
    | Some(message) =>
      <div className="error-banner" role="alert">
        <span> {React.string(message)} </span>
        <button ariaLabel="Dismiss error" onClick={_ => dispatch(ClearError)}>
          {React.string("×")}
        </button>
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
        | GraphView => <GraphView model dispatch />
        }}
      </main>
    </div>
  </div>
}
