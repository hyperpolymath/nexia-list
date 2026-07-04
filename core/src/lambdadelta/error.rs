// SPDX-License-Identifier: MPL-2.0
//! λδ error values.
//!
//! Per the sandbox contract (spec §6) every failure is a *structured value*,
//! never a panic: unbound symbol, arity mismatch, type error, budget-exceeded,
//! and so on. The evaluator surfaces these as `Err(LdError)`; a later layer maps
//! them onto in-notebook error values the UI can show against the offending
//! form. The kernel itself is total — no `unwrap`/`panic` on user input.

use thiserror::Error;

/// A λδ diagnostic. Cheap to clone; carries enough context to point a user at
/// what went wrong.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LdError {
    /// The reader could not make sense of the source text.
    #[error("read error at position {pos}: {msg}")]
    Read { msg: String, pos: usize },

    /// A symbol was evaluated but is not bound in any enclosing scope.
    #[error("unbound symbol: {0}")]
    Unbound(String),

    /// The head of a call form is not something that can be applied.
    #[error("not callable: {0}")]
    NotCallable(String),

    /// A function or special form received the wrong number of arguments.
    #[error("{name}: wrong arity — expected {expected}, got {got}")]
    Arity {
        name: String,
        expected: String,
        got: usize,
    },

    /// A value had the wrong type for the operation.
    #[error("{op}: type error — expected {expected}, got {got}")]
    Type {
        op: String,
        expected: String,
        got: String,
    },

    /// Special-form syntax was malformed (e.g. an odd-length `let` binding).
    #[error("bad syntax in {form}: {msg}")]
    Syntax { form: String, msg: String },

    /// Division (or `mod`) by zero.
    #[error("division by zero")]
    DivideByZero,

    /// The evaluation budget (steps or recursion depth) was exhausted.
    #[error("budget exceeded: {0}")]
    Budget(String),

    /// An error raised deliberately from λδ code.
    #[error("{0}")]
    User(String),
}

/// Convenience alias for kernel results.
pub type LdResult<T> = Result<T, LdError>;
