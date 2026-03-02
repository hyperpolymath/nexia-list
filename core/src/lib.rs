// SPDX-License-Identifier: PMPL-1.0-or-later

//! Nexia Core — Knowledge Graph and Note Engine.
//!
//! This crate provides the foundational data structures for the Nexia 
//! ecosystem. It treats notes as nodes in a multi-dimensional graph, 
//! supporting bidirectional linking and spatial arrangement.
//!
//! ARCHITECTURE:
//! - `note`: Individual atomic unit of information.
//! - `notebook`: A logical collection/subgraph of notes.
//! - `storage`: Content-addressable persistence layer.

pub mod note;
pub mod notebook;
pub mod storage;

// PUBLIC API: Re-export primary types for use in Desktop (Tauri) and Web consumers.
pub use note::{Note, NoteId, Point2D};
pub use notebook::Notebook;
pub use storage::Storage;

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
