;; SPDX-License-Identifier: PMPL-1.0-or-later
(state
  (metadata
    (version "0.1.0")
    (last-updated "2026-03-02")
    (status active))
  (project-context
    (name "nexia-list")
    (purpose "Cross-platform personal knowledge management with spatial notes, relationships, and intelligent agents")
    (completion-percentage 30))
  (components
    (component "rust-core" (status "implemented") (description "Graph engine, search, storage — Note/Notebook/Storage modules"))
    (component "rescript-ui" (status "scaffolded") (description "TEA architecture with Model/View/Update/Msg/Types modules"))
    (component "tauri-desktop" (status "scaffolded") (description "Tauri 2.0 desktop shell with command handlers"))
    (component "web-pwa" (status "scaffolded") (description "HTML/CSS browser entry point"))
    (component "deno-tooling" (status "configured") (description "Deno task runner for dev/build/test/lint")))
  (blockers
    (blocker "No CI for Rust/ReScript builds yet")
    (blocker "Mobile (iOS/Android) shell not started"))
  (critical-next-actions
    (action "Wire Tauri commands to ReScript UI via invoke bridge")
    (action "Add full-text search with tantivy integration")
    (action "Implement spatial canvas drag-and-drop")))
