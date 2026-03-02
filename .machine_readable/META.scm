;; SPDX-License-Identifier: PMPL-1.0-or-later
(meta
  (metadata
    (version "0.1.0")
    (last-updated "2026-03-02"))
  (project-info
    (type application)
    (languages (rust rescript javascript html css))
    (license "PMPL-1.0-or-later")
    (author "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>"))
  (architecture-decisions
    (adr "ADR-001: Use Rust for core engine with serde for serialization"
      (status accepted)
      (rationale "Memory safety, WASM compilation, graph libraries (petgraph), search (tantivy)"))
    (adr "ADR-002: Use ReScript TEA for UI layer"
      (status accepted)
      (rationale "Exhaustive pattern matching, pure update functions, type-safe message passing"))
    (adr "ADR-003: Use Tauri 2.0 for desktop and mobile"
      (status accepted)
      (rationale "Cross-platform from single codebase, small binaries, no Node.js attack surface"))
    (adr "ADR-004: Local-first data storage"
      (status accepted)
      (rationale "User data stays on device, JSON files on desktop, IndexedDB on web"))))
