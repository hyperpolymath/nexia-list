// SPDX-License-Identifier: MPL-2.0
//! The λδ **capability model** — issue #33, foundation 1/2 (spec §7.1).
//!
//! A capability is an unforgeable-by-sandboxing permission token: a plugin
//! declares the capabilities it *requests* in its manifest
//! ([`crate::lambdadelta::package`]), the provisioner computes the grants the
//! user actually *allowed* ([`crate::lambdadelta::provisioner`]), and the host
//! *enforces* them by gating every notebook builtin at registration time
//! ([`crate::lambdadelta_host::register_gated`]). Because enforcement happens
//! in native code before a builtin executes, no λδ code can escape it — the
//! sandbox contract (spec §6) extends from bounded computation to bounded
//! *effect*: nothing runs with capabilities the user hasn't granted.
//!
//! The catalogue is deliberately small at the foundation — whole-notebook
//! scopes — but is named so it can refine later (path/attribute patterns like
//! `:notes/read {:titles "Journal *"}`) without breaking existing manifests.

use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

use super::error::{LdError, LdResult};

/// A permission over host-provided effects. Kept `Copy`-small: enforcement is
/// on the hot path of every host builtin call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Read the notebook: `notes`, `note`, `title`, `content`, `attrs`,
    /// `links`, `backlinks`, `position`, `attr`, `search`, `resolve-title`,
    /// `agents`, and every future pure reader a host registers.
    /// Manifest keyword: `:notes/read`.
    NotesRead,
    /// Mutate the notebook: `create-note!`, `set-title!`, `set-content!`,
    /// `set-attr!`, `remove-attr!`, `move-note!`, `resize-note!`, `link!`,
    /// `unlink!`, `delete-note!`, and every future `!`-suffixed mutator.
    /// Manifest keyword: `:notes/write`.
    NotesWrite,
    /// Run stored agents: `run-agent`. Since an agent's predicate is read code
    /// evaluated over the notebook, holding this capability implies
    /// [`Capability::NotesRead`] (see [`CapabilitySet::allows`]).
    /// Manifest keyword: `:agents/run`.
    AgentsRun,
}

impl Capability {
    /// Every capability the current catalogue defines. A manifest requesting
    /// anything outside this list is rejected — the host cannot grant what it
    /// does not know how to enforce.
    pub const ALL: [Capability; 3] = [
        Capability::NotesRead,
        Capability::NotesWrite,
        Capability::AgentsRun,
    ];

    /// The canonical manifest keyword (`:notes/read`, …).
    pub fn keyword(self) -> &'static str {
        match self {
            Capability::NotesRead => ":notes/read",
            Capability::NotesWrite => ":notes/write",
            Capability::AgentsRun => ":agents/run",
        }
    }

    /// Parse a capability keyword, with or without the leading colon
    /// (`:notes/read` ≡ `notes/read`). Returns `None` for unknown keywords.
    pub fn from_keyword(kw: &str) -> Option<Capability> {
        let kw = kw.strip_prefix(':').unwrap_or(kw);
        match kw {
            "notes/read" => Some(Capability::NotesRead),
            "notes/write" => Some(Capability::NotesWrite),
            "agents/run" => Some(Capability::AgentsRun),
            _ => None,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.keyword())
    }
}

/// A set of granted capabilities. Cloning is cheap when shared as
/// `Rc<CapabilitySet>` — every gated host builtin holds one `Rc`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilitySet {
    granted: BTreeSet<Capability>,
}

impl CapabilitySet {
    /// No grants at all: the plugin gets pure λδ and nothing else.
    pub fn none() -> Self {
        CapabilitySet::default()
    }

    /// Every capability in the catalogue — the grant a fully-trusted (e.g.
    /// first-party, `teranga`-tier) package receives.
    pub fn all() -> Self {
        CapabilitySet {
            granted: Capability::ALL.into_iter().collect(),
        }
    }

    /// Grants built from manifest keywords. Unknown keywords yield
    /// [`LdError::User`] with a hint — surfaced to the provisioner as a
    /// manifest defect, never silently dropped (silently dropping a requested
    /// capability would make a plugin under-powered *and* under-honest about
    /// it: the manifest must reflect what the code needs).
    pub fn from_keywords<I, S>(kws: I) -> LdResult<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut set = CapabilitySet::none();
        for kw in kws {
            let kw = kw.as_ref();
            match Capability::from_keyword(kw) {
                Some(cap) => {
                    set.grant(cap);
                }
                None => {
                    return Err(LdError::User(format!(
                        "unknown capability {kw:?} — catalogue: {}",
                        Capability::ALL
                            .iter()
                            .map(|c| c.keyword())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )));
                }
            }
        }
        Ok(set)
    }

    /// Add a grant.
    pub fn grant(&mut self, cap: Capability) {
        self.granted.insert(cap);
    }

    /// Is `cap` covered by these grants? `AgentsRun` implies `NotesRead`
    /// (running an agent evaluates read code over the notebook), so a grant of
    /// `:agents/run` alone satisfies a `:notes/read` requirement.
    pub fn allows(&self, cap: &Capability) -> bool {
        if self.granted.contains(cap) {
            return true;
        }
        match cap {
            Capability::NotesRead => self.granted.contains(&Capability::AgentsRun),
            _ => false,
        }
    }

    /// Everything in `other` that this set does *not* allow — the list a
    /// provisioner presents to the user as "this package asks for more than
    /// you granted".
    pub fn missing(&self, other: &CapabilitySet) -> Vec<Capability> {
        other
            .granted
            .iter()
            .filter(|cap| !self.allows(cap))
            .copied()
            .collect()
    }

    /// Enforce: succeed iff `cap` is allowed, else an [`LdError::Capability`]
    /// naming the required grant. This is the single choke point every gated
    /// host builtin calls before touching the notebook.
    pub fn require(&self, cap: Capability) -> LdResult<()> {
        if self.allows(&cap) {
            Ok(())
        } else {
            Err(LdError::Capability(format!(
                "this package was not granted {cap}; the manifest must request it and the user must allow it"
            )))
        }
    }

    /// Iterate the granted capabilities (sorted — deterministic reporting).
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.granted.iter()
    }
}

/// Share a grant set across every gated builtin of one sandbox.
pub fn shared(set: CapabilitySet) -> Rc<CapabilitySet> {
    Rc::new(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_roundtrip() {
        for cap in Capability::ALL {
            assert_eq!(Capability::from_keyword(cap.keyword()), Some(cap));
        }
        assert_eq!(
            Capability::from_keyword("notes/read"),
            Some(Capability::NotesRead)
        );
        assert_eq!(Capability::from_keyword(":bogus/read"), None);
    }

    #[test]
    fn none_allows_nothing() {
        let set = CapabilitySet::none();
        assert!(!set.allows(&Capability::NotesRead));
        assert!(set.require(Capability::NotesRead).is_err());
    }

    #[test]
    fn agents_run_implies_notes_read() {
        let set = CapabilitySet::from_keywords([":agents/run"]).unwrap();
        assert!(set.allows(&Capability::NotesRead));
        assert!(!set.allows(&Capability::NotesWrite));
    }

    #[test]
    fn missing_reports_exactly_the_delta() {
        let granted = CapabilitySet::from_keywords([":notes/read"]).unwrap();
        let wanted = CapabilitySet::from_keywords([":notes/read", ":notes/write"]).unwrap();
        assert_eq!(granted.missing(&wanted), vec![Capability::NotesWrite]);
    }

    #[test]
    fn unknown_keyword_is_a_manifest_error_not_a_silent_drop() {
        let err = CapabilitySet::from_keywords([":disk/write"]).unwrap_err();
        assert!(format!("{err}").contains("unknown capability"));
    }
}
