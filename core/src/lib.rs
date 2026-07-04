// SPDX-License-Identifier: MPL-2.0

//! Nexia-List Core — Knowledge Graph and Note Engine.
//!
//! This crate provides the foundational data structures for the Nexia-List
//! ecosystem. It treats notes as nodes in a multi-dimensional graph,
//! supporting bidirectional linking and spatial arrangement.
//!
//! ARCHITECTURE:
//! - `note`: Individual atomic unit of information.
//! - `notebook`: A logical collection/subgraph of notes.
//! - `storage`: File-based JSON persistence.
//! - `wikilink`: `[[Title]]` reference parsing.
//! - `exchange`: Markdown-vault / OPML import & export.
//! - `agent`: persistent saved queries (the app's namesake feature).
//! - `lambdadelta` (λδ): the programmable substrate — a small homoiconic Lisp
//!   whose kernel knows nothing about notes (a host registers notebook
//!   builtins). Opt-in and invisible by default (see ADR-0003).
//! - `lambdadelta_host`: the notebook host for λδ — registers the note-aware
//!   builtins into a kernel through the seam. This is where λδ touches notes;
//!   the kernel stays note-agnostic.
//! - `wasm` (feature "wasm"): browser bindings; the web UI runs this crate
//!   compiled to WebAssembly as its single source of truth.

#![forbid(unsafe_code)]
pub mod agent;
pub mod exchange;
pub mod lambdadelta;
pub mod lambdadelta_host;
pub mod note;
pub mod notebook;
pub mod storage;
pub mod wikilink;

#[cfg(feature = "wasm")]
pub mod wasm;

// PUBLIC API: Re-export primary types for the desktop shell and web consumers.
pub use note::{Note, NoteId, Point2D};
pub use notebook::Notebook;
pub use storage::Storage;

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
