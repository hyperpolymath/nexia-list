// SPDX-License-Identifier: MPL-2.0
//! LambdaDelta (λδ) — the programmable substrate for Nexia-List.
//!
//! > *A note is a letter we send to our future self.* — *The Tinderbox Way*
//!
//! λδ is a small, homoiconic, Clojure-flavoured Lisp implemented in the Rust
//! core (and compiled to WASM). It exists to *enlarge* the correspondence the
//! North Star names — power in service of the letter, opt-in and invisible by
//! default. See [ADR-0003](../../../docs/adr/0003-lambdadelta-lisp-substrate.md)
//! and the [spec](../../../docs/design/lambdadelta-spec.md).
//!
//! # The kernel / host seam (spec §7)
//!
//! This module is the **kernel**: reader, value model, evaluator, budget, and a
//! small set of *pure* builtins. It **knows nothing about notes**. A host (the
//! notebook — the first of possibly many) registers its own builtins through
//! [`Interp::register_builtin`], each a plain native function. That single
//! discipline is what makes an SDK, embedding, and the plugin ecosystem (#33)
//! cheap rather than a later rewrite.
//!
//! Phase L0 delivers the kernel foundation: reader + value model + evaluator
//! for the core special forms + budget + pure builtins. Hygienic macros,
//! multimethods, and the notebook host bindings layer on top without changing
//! the seam.

mod builtins;
mod error;
mod eval;
mod reader;
mod value;

pub use error::{LdError, LdResult};
pub use reader::{read_all, read_one};
pub use value::{Builtin, BuiltinImpl, Closure, Env, Scope, Value};

use std::rc::Rc;

/// A resource budget for one evaluation (spec §6). Bounds keep user code from
/// hanging the tab: a reduction-step ceiling and a recursion-depth limit. Both
/// abort cleanly with an [`LdError::Budget`] value.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    /// Reduction steps remaining (each `eval`/`apply` costs one).
    pub steps: u64,
    /// Maximum call-stack depth.
    pub max_depth: usize,
    /// Current depth (internal bookkeeping; set at construction to 0).
    depth: usize,
}

impl Budget {
    /// A generous default suitable for interactive formulas and agent queries.
    pub fn new() -> Self {
        Budget {
            steps: 1_000_000,
            max_depth: 512,
            depth: 0,
        }
    }

    /// A budget with an explicit step ceiling and depth limit.
    pub fn with_limits(steps: u64, max_depth: usize) -> Self {
        Budget {
            steps,
            max_depth,
            depth: 0,
        }
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget::new()
    }
}

/// The λδ interpreter: a global environment plus the current evaluation budget.
///
/// Construct with [`Interp::new`] (kernel builtins only), register host builtins
/// with [`register_builtin`](Interp::register_builtin), then evaluate source
/// with [`eval_str`](Interp::eval_str).
pub struct Interp {
    /// The top-level (notebook) environment. `def` binds here.
    pub global: Env,
    pub(crate) budget: Budget,
}

impl Interp {
    /// A fresh interpreter with only the kernel's pure builtins installed. No
    /// notebook access — that is a host's job to register.
    pub fn new() -> Self {
        let global = Scope::root();
        let mut interp = Interp {
            global,
            budget: Budget::new(),
        };
        builtins::install(&mut interp);
        interp
    }

    /// Register a native function into the global environment — **the host
    /// seam**. `max_arity` of `None` means variadic. The closure receives the
    /// interpreter (for `apply`/budget and any captured host state) and the
    /// already-evaluated arguments.
    ///
    /// ```
    /// use nexia_core::lambdadelta::{Interp, Value, Budget};
    /// let mut interp = Interp::new();
    /// interp.register_builtin("double", 1, Some(1), |_i, args| {
    ///     match &args[0] {
    ///         Value::Int(n) => Ok(Value::Int(n * 2)),
    ///         other => Ok(other.clone()),
    ///     }
    /// });
    /// let out = interp.eval_str("(double 21)", Budget::new()).unwrap();
    /// assert_eq!(out, Value::Int(42));
    /// ```
    pub fn register_builtin<F>(
        &mut self,
        name: &str,
        min_arity: usize,
        max_arity: Option<usize>,
        func: F,
    ) where
        F: Fn(&mut Interp, &[Value]) -> LdResult<Value> + 'static,
    {
        let name: Rc<str> = Rc::from(name);
        let builtin = Builtin {
            name: name.clone(),
            min_arity,
            max_arity,
            func: Box::new(func),
        };
        self.global.set(name, Value::Builtin(Rc::new(builtin)));
    }

    /// Read and evaluate `src` under `budget`, returning the value of the final
    /// top-level form (or `nil` if there are none). Each call runs under a fresh
    /// budget; the global environment persists across calls (so `def`s stick).
    pub fn eval_str(&mut self, src: &str, budget: Budget) -> LdResult<Value> {
        let forms = read_all(src)?;
        self.budget = budget;
        let global = self.global.clone();
        let mut result = Value::Nil;
        for form in &forms {
            result = self.eval(form, &global)?;
        }
        Ok(result)
    }
}

impl Default for Interp {
    fn default() -> Self {
        Interp::new()
    }
}

#[cfg(test)]
mod tests;
