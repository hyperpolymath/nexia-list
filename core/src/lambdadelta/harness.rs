// SPDX-License-Identifier: MPL-2.0
//! The λδ **harness** — issue #33. The sandboxed develop/test environment a
//! plugin author iterates in: run the package against a fixture notebook, with
//! the evaluator's budget *and* capability enforcement active, so "works on my
//! machine" means "works inside the sandbox it will ship in". CI verifies
//! plugins with the same harness (no second implementation).
//!
//! Design notes
//! ────────────
//! * The harness is registered by *closure*: it builds an [`Interp`] and hands
//!   it to a caller-supplied registrar, so the kernel never depends on any
//!   host. A notebook host passes
//!   `|i| lambdadelta_host::register_gated(i, nb, grants)`; a pure-language
//!   package passes nothing ([`Harness::pure`]).
//! * Assertions are **recorded, not thrown**: `assert-eq` inside a `.ld` test
//!   file appends to a report instead of aborting at the first failure, so an
//!   author sees the whole damage, and a reader/evaluator error in the test
//!   file becomes a failed assertion rather than a panic (sandbox contract,
//!   spec §6: failures are structured values, never panics).

use std::cell::RefCell;
use std::rc::Rc;

use super::error::LdResult;
use super::value::Value;
use super::{Budget, Interp};

/// One recorded assertion. Strings (not values) keep the report easy to print,
/// diff, and serialise for CI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assertion {
    pub ok: bool,
    pub want: String,
    pub got: String,
}

/// A full test run: every assertion, in order, plus tallies.
#[derive(Clone, Debug)]
pub struct HarnessReport {
    pub assertions: Vec<Assertion>,
    pub passed: usize,
    pub failed: usize,
}

impl HarnessReport {
    /// Every assertion passed (including vacuously — a test file with no
    /// assertions is trivially green; the minter's skeleton always ships one).
    pub fn is_green(&self) -> bool {
        self.failed == 0
    }
}

/// A sandboxed λδ test environment for one package.
pub struct Harness {
    interp: Interp,
    budget: Budget,
    assertions: Rc<RefCell<Vec<Assertion>>>,
}

impl Harness {
    /// Build a sandbox whose host surface is whatever `register` installs
    /// (typically [`crate::lambdadelta_host::register_gated`] with the granted
    /// capabilities from the install plan). The assertion builtins
    /// (`assert-eq`, `assert`) are always installed on top; they record into
    /// the shared buffer that [`Harness::report`] reads.
    pub fn new(register: impl FnOnce(&mut Interp)) -> Self {
        let mut interp = Interp::new();
        register(&mut interp);
        let assertions: Rc<RefCell<Vec<Assertion>>> = Rc::new(RefCell::new(Vec::new()));

        let rec = assertions.clone();
        interp.register_builtin("assert-eq", 2, Some(2), move |_i, a| {
            let want = a[0].to_string();
            let got = a[1].to_string();
            let ok = want == got;
            rec.borrow_mut().push(Assertion { ok, want, got });
            Ok(Value::Bool(ok))
        });

        let rec = assertions.clone();
        interp.register_builtin("assert", 1, Some(1), move |_i, a| {
            let ok = a[0].is_truthy();
            rec.borrow_mut().push(Assertion {
                ok,
                want: "truthy".to_string(),
                got: a[0].to_string(),
            });
            Ok(Value::Bool(ok))
        });

        Harness {
            interp,
            budget: Budget::new(),
            assertions,
        }
    }

    /// A kernel-only harness: pure λδ, no host builtins at all. Right for
    /// packages that compute (no notebook effects) and for testing the
    /// kernel-facing parts of effectful packages.
    pub fn pure() -> Self {
        Harness::new(|_| {})
    }

    /// Override the default budget (1M steps / depth 512), e.g. to give a
    /// community-tier package a deliberately tight leash in CI.
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// Direct access to the sandbox (e.g. for the wizard's REPL later).
    pub fn interp(&mut self) -> &mut Interp {
        &mut self.interp
    }

    /// Load package *source* (definitions). Errors are returned: code that
    /// cannot even be read/defined is not a test failure, it is a broken
    /// package — the author should fix the file, not read a red assertion.
    pub fn load_source(&mut self, _label: &str, src: &str) -> LdResult<()> {
        self.interp.eval_str(src, self.budget).map(|_| ())
    }

    /// Run a `.ld` *test* file. Never returns `Err`: assertions recorded by
    /// `assert-eq`/`assert`, and a read/eval error mid-file becomes one failed
    /// assertion (reporting the error value) so the report stays complete and
    /// nothing panics across the WASM boundary.
    pub fn run_tests(&mut self, label: &str, src: &str) {
        match self.interp.eval_str(src, self.budget) {
            Ok(_) => {}
            Err(e) => self.assertions.borrow_mut().push(Assertion {
                ok: false,
                want: format!("{label} evaluates to completion"),
                got: format!("{e}"),
            }),
        }
    }

    /// Tally everything recorded so far.
    pub fn report(&self) -> HarnessReport {
        let assertions = self.assertions.borrow();
        let passed = assertions.iter().filter(|a| a.ok).count();
        HarnessReport {
            failed: assertions.len() - passed,
            passed,
            assertions: assertions.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_harness_records_assertions() {
        let mut h = Harness::pure();
        h.load_source("defs", "(def double (fn [x] (* 2 x)))")
            .unwrap();
        h.run_tests(
            "tests",
            "(assert-eq 4 (double 2)) (assert-eq 5 (double 2)) (assert true)",
        );
        let r = h.report();
        assert_eq!((r.passed, r.failed), (2, 1));
        assert!(!r.is_green());
    }

    #[test]
    fn evaluation_errors_become_failures_not_panics() {
        let mut h = Harness::pure();
        h.run_tests("bad", "(this-symbol/is-not-bound 1)");
        let r = h.report();
        assert_eq!(r.failed, 1);
        assert!(r.assertions[0].got.contains("unbound"));
    }
}
