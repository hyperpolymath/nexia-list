// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nexia Core - Knowledge graph engine
//!
//! This crate provides the core data structures and operations for Nexia,
//! a cross-platform personal knowledge management tool.

pub mod note;
pub mod notebook;
pub mod storage;

pub use note::{Note, NoteId, Point2D};
pub use notebook::Notebook;
pub use storage::Storage;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
