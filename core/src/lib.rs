// SPDX-License-Identifier: MPL-2.0

//! Nexia Core — Knowledge Graph and Note Engine.
//!
//! This crate provides the foundational data structures for the Nexia
//! ecosystem. It treats notes as nodes in a multi-dimensional graph,
//! supporting bidirectional linking and spatial arrangement.
//!
//! ARCHITECTURE:
//! - `note`: Individual atomic unit of information.
//! - `notebook`: A logical collection/subgraph of notes.
//! - `storage`: File-based JSON persistence.
//! - `wasm` (feature "wasm"): browser bindings; the web UI runs this crate
//!   compiled to WebAssembly as its single source of truth.

#![forbid(unsafe_code)]
pub mod note;
pub mod notebook;
pub mod storage;

#[cfg(feature = "wasm")]
pub mod wasm;

// PUBLIC API: Re-export primary types for the desktop shell and web consumers.
pub use note::{Note, NoteId, Point2D};
pub use notebook::Notebook;
pub use storage::Storage;

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
