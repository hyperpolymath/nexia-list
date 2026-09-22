// SPDX-License-Identifier: MPL-2.0
//! The λδ **provisioner** — issue #33. Installs a package into a notebook/host:
//! declared dependencies, the **capability grants** it requests, and version
//! pinning. The foundational half is *pure*: given a validated manifest, the
//! grants the user is willing to make, and any config overrides, compute an
//! [`InstallPlan`] — or refuse. Persistence (`InstallReceipt` in notebook
//! storage) and the UI prompt are the host's half, layered on top.
//!
//! The one rule that can never be bent (spec §7.1, issue #33):
//!
//! > **Nothing runs with capabilities the user hasn't granted.**
//!
//! So `plan_install` fails — it never silently narrows — when the manifest
//! requests more than the offered grants. Denial is total, not partial: a
//! package that gets 80% of what it declared is a package running code the
//! author wrote for 100%, which breaks in ways the user cannot predict.

use std::fmt;

use thiserror::Error;

use super::capability::{Capability, CapabilitySet};
use super::package::{ManifestError, PackageManifest, Tier};
use super::value::Value;

/// A fully-resolved installation: what will run, with which grants, under
/// which configuration. The host executes the plan by building a
/// [`crate::lambdadelta::harness::Harness`] or live interpreter with
/// `plan.granted` and loading `plan.entry_point`.
#[derive(Clone, Debug)]
pub struct InstallPlan {
    pub name: String,
    pub version: String,
    pub tier: Tier,
    /// Exactly the grants the sandbox will enforce — the *intersection check*
    /// passed, so this is everything the manifest requested, no more.
    pub granted: CapabilitySet,
    /// Fully-resolved configuration (defaults + validated overrides).
    pub config: Vec<(String, Value)>,
    /// Package-relative path of the entry point to load.
    pub entry_point: String,
    /// Package-relative test files the harness can verify before first run.
    pub tests: Vec<String>,
}

/// Why an installation was refused.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum ProvisionError {
    /// The manifest itself is invalid.
    #[error("invalid manifest: {0}")]
    Manifest(#[from] ManifestError),
    /// The package requests capabilities beyond the offered grants. The
    /// `missing` list is exactly what the user would additionally have to
    /// allow (or the author would have to stop requesting).
    #[error("capabilities requested but not granted: {}", .missing.iter().map(|c| c.keyword()).collect::<Vec<_>>().join(" "))]
    CapabilityDenied { missing: Vec<Capability> },
}

/// Compute an install plan. `offered` is what the user consents to grant
/// (their answer to the provisioner prompt); `config_overrides` are
/// configurator input atop the manifest defaults.
pub fn plan_install(
    manifest: &PackageManifest,
    offered: &CapabilitySet,
    config_overrides: &[(String, Value)],
) -> Result<InstallPlan, ProvisionError> {
    let missing = offered.missing(&manifest.requested);
    if !missing.is_empty() {
        return Err(ProvisionError::CapabilityDenied { missing });
    }
    let config = manifest.resolve_config(config_overrides)?;
    Ok(InstallPlan {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        tier: manifest.tier,
        // Grant exactly what was requested — never more. Least privilege is
        // not a posture added later; it is the data the sandbox receives.
        granted: manifest.requested.clone(),
        config,
        entry_point: manifest.entry_point.clone(),
        tests: manifest.tests.clone(),
    })
}

impl fmt::Display for InstallPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{} ({}) — grants: {}",
            self.name,
            self.version,
            self.tier,
            self.granted
                .iter()
                .map(|c| c.keyword())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PackageManifest {
        PackageManifest::from_source(
            r#"{:name "word-count"
                :version "0.1.0"
                :entry-point "src/main.ld"
                :capabilities [:notes/read :notes/write]
                :config {:min-words {:type :int :default 0}}}"#,
        )
        .unwrap()
    }

    #[test]
    fn full_grants_install() {
        let plan = plan_install(&manifest(), &CapabilitySet::all(), &[]).unwrap();
        assert!(plan.granted.allows(&Capability::NotesWrite));
        assert_eq!(plan.config.len(), 1);
        assert_eq!(
            format!("{plan}"),
            "word-count@0.1.0 (ayo) — grants: :notes/read :notes/write"
        );
    }

    #[test]
    fn partial_grants_are_refused_totally() {
        let read_only = CapabilitySet::from_keywords([":notes/read"]).unwrap();
        match plan_install(&manifest(), &read_only, &[]) {
            Err(ProvisionError::CapabilityDenied { missing }) => {
                assert_eq!(missing, vec![Capability::NotesWrite]);
            }
            other => panic!("expected CapabilityDenied, got {other:?}"),
        }
    }

    #[test]
    fn grants_are_exactly_what_was_requested() {
        let plan = plan_install(&manifest(), &CapabilitySet::all(), &[]).unwrap();
        assert!(!plan.granted.allows(&Capability::AgentsRun));
    }

    #[test]
    fn bad_overrides_fail_install() {
        let err = plan_install(
            &manifest(),
            &CapabilitySet::all(),
            &[("min-words".into(), Value::Bool(true))],
        );
        assert!(matches!(err, Err(ProvisionError::Manifest(_))));
    }
}
