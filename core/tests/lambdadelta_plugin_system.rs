// SPDX-License-Identifier: MPL-2.0
//! End-to-end proof of the λδ plugin-system foundation (issue #33):
//! manifest → provisioner → harness, with capability enforcement proven at
//! the host seam. The fixture package is `plugins/word-count/` — itself
//! minted by `scripts/ld-mint.js`, so the minter's output is exercised here
//! too (four components dogfooding one foundation).
//!
//! The fixture notebook mirrors `plugins/word-count/test/main.test.ld`:
//!   Alpha = "one two three four five" → 5 words
//!   Beta  = ""                        → 0 words
//!   Gamma = "just three here"         → 3 words

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use nexia_core::lambdadelta::{
    plan_install, Budget, CapabilitySet, Harness, Interp, PackageManifest, ProvisionError, Tier,
    Value,
};
use nexia_core::lambdadelta_host;
use nexia_core::notebook::Notebook;

fn package_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/word-count")
}

fn fixture_notebook() -> Notebook {
    let mut nb = Notebook::new("plugin-fixture");
    let alpha = nb.create_note("Alpha");
    nb.set_content(&alpha, "one two three four five");
    let beta = nb.create_note("Beta");
    nb.set_content(&beta, "");
    let gamma = nb.create_note("Gamma");
    nb.set_content(&gamma, "just three here");
    nb
}

fn read_to_string(rel: &str) -> String {
    std::fs::read_to_string(package_dir().join(rel))
        .unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

#[test]
fn reference_package_manifest_validates() {
    let m = PackageManifest::from_source(&read_to_string("manifest.ld")).unwrap();
    assert_eq!(m.name, "word-count");
    assert_eq!(m.tier, Tier::Ayo);
    assert_eq!(m.entry_point, "src/main.ld");
    assert!(m
        .requested
        .allows(&nexia_core::lambdadelta::Capability::NotesRead));
    assert!(m
        .requested
        .allows(&nexia_core::lambdadelta::Capability::NotesWrite));
    assert_eq!(m.tests, vec!["test/main.test.ld".to_string()]);
}

#[test]
fn mint_manifest_provisions_over_full_grants_and_runs_green() {
    let manifest = PackageManifest::from_source(&read_to_string("manifest.ld")).unwrap();
    let plan = plan_install(&manifest, &CapabilitySet::all(), &[]).expect("provisioning");

    let nb = Rc::new(RefCell::new(fixture_notebook()));
    let grants = Rc::new(plan.granted.clone());
    let nb_for_sandbox = nb.clone();
    let mut harness = Harness::new(move |interp| {
        lambdadelta_host::register_gated(interp, nb_for_sandbox, grants);
    });

    harness
        .load_source("src/main.ld", &read_to_string("src/main.ld"))
        .expect("package source loads");
    harness.run_tests("test/main.test.ld", &read_to_string("test/main.test.ld"));

    let report = harness.report();
    assert!(
        report.is_green(),
        "harness report must be green; failures: {:?}",
        report
            .assertions
            .iter()
            .filter(|a| !a.ok)
            .collect::<Vec<_>>()
    );
    assert_eq!(report.passed, 7, "expected all 7 assertions to pass");

    // And the mutation really happened in the host notebook.
    let nb = nb.borrow();
    let alpha = nb.search_by_title("Alpha")[0];
    assert_eq!(
        alpha.get_attribute("word-count"),
        Some(&serde_json::json!(5))
    );
}

#[test]
fn provisioner_refuses_partial_grants_totally() {
    let manifest = PackageManifest::from_source(&read_to_string("manifest.ld")).unwrap();
    let read_only = CapabilitySet::from_keywords([":notes/read"]).unwrap();
    match plan_install(&manifest, &read_only, &[]) {
        Err(ProvisionError::CapabilityDenied { missing }) => {
            assert_eq!(
                missing,
                vec![nexia_core::lambdadelta::Capability::NotesWrite]
            );
        }
        other => panic!("expected total refusal, got {other:?}"),
    }
}

#[test]
fn sandbox_enforces_read_only_grants_with_structured_denial() {
    // Even if a provisioner bug somehow let a write-requiring package load,
    // the HOST seam still denies the effect (defence in depth).
    let nb = Rc::new(RefCell::new(fixture_notebook()));
    let read_only = Rc::new(CapabilitySet::from_keywords([":notes/read"]).unwrap());
    let nb_for_sandbox = nb.clone();
    let grants = read_only.clone();
    let mut interp = Interp::new();
    lambdadelta_host::register_gated(&mut interp, nb_for_sandbox, grants);

    // Reads work.
    let out = interp.eval_str("(count (notes))", Budget::new()).unwrap();
    assert!(matches!(out, Value::Int(3)));

    // The package's mutating entry point fails with LdError::Capability —
    // a structured value, not a panic, and the notebook is untouched.
    interp
        .eval_str(&read_to_string("src/main.ld"), Budget::new())
        .expect("definitions under read grants load fine (no effects at def time)");
    let err = interp
        .eval_str("(annotate-word-counts!)", Budget::new())
        .unwrap_err();
    assert!(
        matches!(err, nexia_core::lambdadelta::LdError::Capability(_)),
        "expected capability denial, got {err:?}"
    );

    let nb = nb.borrow();
    let alpha = nb.search_by_title("Alpha")[0];
    assert_eq!(alpha.get_attribute("word-count"), None);
}

#[test]
fn agents_run_is_not_implied_by_notes_read() {
    let nb = Rc::new(RefCell::new(fixture_notebook()));
    let grants = Rc::new(CapabilitySet::from_keywords([":notes/read"]).unwrap());
    let nb_for_sandbox = nb.clone();
    let g = grants.clone();
    let mut interp = Interp::new();
    lambdadelta_host::register_gated(&mut interp, nb_for_sandbox, g);

    let err = interp
        .eval_str("(run-agent \"anything\")", Budget::new())
        .unwrap_err();
    assert!(
        matches!(err, nexia_core::lambdadelta::LdError::Capability(_)),
        "run-agent must require :agents/run, got {err:?}"
    );
}
