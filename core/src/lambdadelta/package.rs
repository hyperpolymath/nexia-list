// SPDX-License-Identifier: MPL-2.0
//! The λδ **package format** — issue #33, foundation 2/2.
//!
//! A λδ package (conventionally a `<name>/` directory under `plugins/`, shared
//! as a `<name>.ldpkg` bundle) is:
//!
//! ```text
//! word-count/
//! ├── manifest.ld        ; this file's format — a single λδ map (code is data)
//! ├── src/main.ld        ; the entry point named by :entry-point
//! ├── test/main.test.ld  ; harness tests named by :tests
//! └── README.adoc        ; human docs (Asciidoc, estate standard)
//! ```
//!
//! The manifest is *homoiconic*: it is a λδ map literal parsed by the ordinary
//! λδ reader — no second parser, no schema language, no dependency. Field
//! names deliberately mirror the BoJ `cartridge.json` conventions
//! (`name`/`version`/`spdx`/`tier`/`description`) and the PanLL
//! minter/provisioner contracts, so mapping between ecosystems is mechanical
//! (see `docs/design/lambdadelta-plugin-system.adoc`).
//!
//! ```clojure
//! {:name "word-count"
//!  :version "0.1.0"
//!  :spdx "MPL-2.0"
//!  :tier :ayo                      ; :teranga core | :shield elevated-trust | :ayo community
//!  :description "Counts words per note and writes the :word-count attribute"
//!  :entry-point "src/main.ld"
//!  :capabilities [:notes/read :notes/write]   ; requested grants — see capability.rs
//!  :config {:min-words {:type :int :default 0 :doc "ignore shorter notes"}}
//!  :tests ["test/main.test.ld"]}
//! ```

use std::fmt;

use thiserror::Error;

use super::capability::{Capability, CapabilitySet};
use super::reader::read_one;
use super::value::Value;

/// Trust tier, mirroring `panll/src/abi/cartridge-schema.json` so ecosystem
/// tooling understands λδ packages without new vocabulary:
/// `Teranga` = core, always available · `Shield` = security-critical, elevated
/// trust · `Ayo` = community-contributed (the default for minted packages).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Teranga,
    Shield,
    #[default]
    Ayo,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Teranga => "teranga",
            Tier::Shield => "shield",
            Tier::Ayo => "ayo",
        }
    }

    fn from_name(name: &str) -> Result<Tier, ManifestError> {
        match name {
            "teranga" => Ok(Tier::Teranga),
            "shield" => Ok(Tier::Shield),
            "ayo" => Ok(Tier::Ayo),
            other => Err(ManifestError::Field {
                field: ":tier".into(),
                msg: format!("unknown tier {other:?} — one of :teranga :shield :ayo"),
            }),
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The declared type of one configuration setting (the configurator's schema).
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigType {
    String,
    Int,
    Float,
    Bool,
    Keyword,
    /// A keyword from a fixed set of allowed names: `{:type :enum :values [...]}`.
    Enum(Vec<String>),
}

impl ConfigType {
    fn name(&self) -> String {
        match self {
            ConfigType::String => ":string".into(),
            ConfigType::Int => ":int".into(),
            ConfigType::Float => ":float".into(),
            ConfigType::Bool => ":bool".into(),
            ConfigType::Keyword => ":keyword".into(),
            ConfigType::Enum(allowed) => format!("(:enum {})", allowed.join(" ")),
        }
    }

    fn check(&self, key: &str, v: &Value) -> Result<(), ManifestError> {
        let ok = match (self, v) {
            (ConfigType::String, Value::Str(_)) => true,
            (ConfigType::Int, Value::Int(_)) => true,
            (ConfigType::Float, Value::Float(_)) => true,
            (ConfigType::Bool, Value::Bool(_)) => true,
            (ConfigType::Keyword, Value::Keyword(_)) => true,
            (ConfigType::Enum(allowed), Value::Keyword(k)) => {
                allowed.iter().any(|a| a == k.as_ref())
            }
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(ManifestError::Field {
                field: format!(":config :{key}"),
                msg: format!("value {v} does not match declared type {}", self.name()),
            })
        }
    }
}

/// One declared configuration setting: type, optional default, optional doc.
#[derive(Clone, Debug)]
pub struct ConfigSpec {
    pub name: String,
    pub ty: ConfigType,
    pub default: Option<Value>,
    pub doc: Option<String>,
}

/// A validated package manifest.
#[derive(Clone, Debug)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub spdx: Option<String>,
    pub tier: Tier,
    pub description: Option<String>,
    /// Entry-point source file, package-relative (e.g. `src/main.ld`).
    pub entry_point: String,
    /// Capabilities the package *requests*. The provisioner intersects these
    /// with the user's grants; enforcement is in the host.
    pub requested: CapabilitySet,
    /// Declared configuration schema (the configurator's UI surface).
    pub config: Vec<ConfigSpec>,
    /// Test files (harness inputs), package-relative.
    pub tests: Vec<String>,
}

/// Everything that can be wrong with a manifest. Deliberately field-shaped so
/// provisioner/configurator UIs can point the author at the exact key.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum ManifestError {
    #[error("manifest read error: {0}")]
    Read(String),
    #[error("manifest must be a single λδ map literal, got: {0}")]
    NotAMap(String),
    #[error("manifest field {field}: {msg}")]
    Field { field: String, msg: String },
}

fn field_err<T>(field: &str, msg: impl Into<String>) -> Result<T, ManifestError> {
    Err(ManifestError::Field {
        field: field.into(),
        msg: msg.into(),
    })
}

/// Look up `key` (a bare name like `"name"`) in a λδ keyword-keyed map.
fn map_get<'m>(map: &'m [(Value, Value)], key: &str) -> Option<&'m Value> {
    map.iter().find_map(|(k, v)| match k {
        Value::Keyword(kw) if kw.as_ref() == key => Some(v),
        _ => None,
    })
}

fn want_string<'m>(map: &'m [(Value, Value)], key: &str) -> Result<&'m str, ManifestError> {
    match map_get(map, key) {
        Some(Value::Str(s)) => Ok(s.as_ref()),
        Some(other) => field_err(
            &format!(":{key}"),
            format!("expected a string, got {other}"),
        ),
        None => field_err(&format!(":{key}"), "required field is missing"),
    }
}

fn opt_string(map: &[(Value, Value)], key: &str) -> Result<Option<String>, ManifestError> {
    match map_get(map, key) {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.to_string())),
        Some(other) => field_err(
            &format!(":{key}"),
            format!("expected a string, got {other}"),
        ),
    }
}

/// kebab-case: the minter, BoJ cartridges, and npm-adjacent tooling all agree.
fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

/// `x.y.z` (loosely — numeric components, no build metadata requirements).
fn valid_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

impl PackageManifest {
    /// Parse and validate a manifest from its source text.
    pub fn from_source(src: &str) -> Result<PackageManifest, ManifestError> {
        let form = read_one(src).map_err(|e| ManifestError::Read(format!("{e}")))?;
        PackageManifest::from_value(&form)
    }

    /// Validate a manifest given as an already-read λδ value (the homoiconic
    /// path: a host may obtain the map from anywhere — a `.ld` file, an
    /// in-notebook attachment, a registry response).
    pub fn from_value(v: &Value) -> Result<PackageManifest, ManifestError> {
        let map = match v {
            Value::Map(pairs) => pairs.as_ref(),
            other => return Err(ManifestError::NotAMap(format!("{other}"))),
        };

        let name = want_string(map, "name")?.to_string();
        if !valid_package_name(&name) {
            return field_err(
                ":name",
                format!("{name:?} is not kebab-case (a-z, 0-9, single dashes)"),
            );
        }

        let version = want_string(map, "version")?.to_string();
        if !valid_version(&version) {
            return field_err(":version", format!("{version:?} — expected x.y.z"));
        }

        let spdx = opt_string(map, "spdx")?;
        let description = opt_string(map, "description")?;

        let tier = match map_get(map, "tier") {
            None | Some(Value::Nil) => Tier::default(),
            Some(Value::Keyword(k)) => Tier::from_name(k.as_ref())?,
            Some(Value::Str(s)) => Tier::from_name(s.as_ref())?,
            Some(other) => {
                return field_err(":tier", format!("expected a keyword, got {other}"));
            }
        };

        let entry_point = want_string(map, "entry-point")?.to_string();
        if entry_point.contains("..") || entry_point.starts_with('/') {
            return field_err(
                ":entry-point",
                "must be a package-relative path (no `..`, not absolute)",
            );
        }

        let requested = match map_get(map, "capabilities") {
            None | Some(Value::Nil) => CapabilitySet::none(),
            Some(Value::Vector(items)) | Some(Value::List(items)) => {
                let mut set = CapabilitySet::none();
                for item in items.iter() {
                    let kw = match item {
                        Value::Keyword(k) => k.to_string(),
                        Value::Str(s) => s.to_string(),
                        other => {
                            return field_err(
                                ":capabilities",
                                format!("expected capability keywords, got {other}"),
                            );
                        }
                    };
                    match Capability::from_keyword(&kw) {
                        Some(cap) => set.grant(cap),
                        None => {
                            return field_err(
                                ":capabilities",
                                format!("unknown capability {kw}; the host cannot grant what it cannot enforce"),
                            );
                        }
                    }
                }
                set
            }
            Some(other) => {
                return field_err(":capabilities", format!("expected a vector, got {other}"));
            }
        };

        let config = match map_get(map, "config") {
            None | Some(Value::Nil) => Vec::new(),
            Some(Value::Map(entries)) => parse_config(entries.as_ref())?,
            Some(other) => {
                return field_err(":config", format!("expected a map, got {other}"));
            }
        };

        let tests = match map_get(map, "tests") {
            None | Some(Value::Nil) => Vec::new(),
            Some(Value::Vector(items)) | Some(Value::List(items)) => {
                let mut out = Vec::new();
                for item in items.iter() {
                    match item {
                        Value::Str(s) => out.push(s.to_string()),
                        other => {
                            return field_err(":tests", format!("expected strings, got {other}"));
                        }
                    }
                }
                out
            }
            Some(other) => return field_err(":tests", format!("expected a vector, got {other}")),
        };

        Ok(PackageManifest {
            name,
            version,
            spdx,
            tier,
            description,
            entry_point,
            requested,
            config,
            tests,
        })
    }

    /// Resolve configuration: apply `overrides` on top of defaults, type-check
    /// every resulting value against the declared schema, reject unknown keys.
    /// Returns the fully-resolved settings the configuration step would
    /// persist. This is the configurator's enforcement half — a declared
    /// schema the UI is generated from, and the guarantee that no unvalidated
    /// value reaches the plugin.
    pub fn resolve_config(
        &self,
        overrides: &[(String, Value)],
    ) -> Result<Vec<(String, Value)>, ManifestError> {
        for (key, _) in overrides {
            if !self.config.iter().any(|spec| &spec.name == key) {
                return field_err(
                    &format!(":config :{key}"),
                    "no such declared setting — the manifest's :config schema is the boundary",
                );
            }
        }
        let mut resolved = Vec::new();
        for spec in &self.config {
            let value = overrides
                .iter()
                .find_map(|(k, v)| {
                    if k == &spec.name {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
                .or_else(|| spec.default.clone());
            match value {
                Some(v) => {
                    spec.ty.check(&spec.name, &v)?;
                    resolved.push((spec.name.clone(), v));
                }
                None => {
                    return field_err(
                        &format!(":config :{}", spec.name),
                        "no default and no override — this setting is required",
                    );
                }
            }
        }
        Ok(resolved)
    }
}

fn parse_config(entries: &[(Value, Value)]) -> Result<Vec<ConfigSpec>, ManifestError> {
    let mut out = Vec::new();
    for (k, v) in entries {
        let name = match k {
            Value::Keyword(kw) => kw.to_string(),
            other => return field_err(":config", format!("keys must be keywords, got {other}")),
        };
        let spec_map = match v {
            Value::Map(m) => m.as_ref(),
            other => {
                return field_err(
                    &format!(":config :{name}"),
                    format!("expected a spec map, got {other}"),
                );
            }
        };
        let ty = match map_get(spec_map, "type") {
            Some(Value::Keyword(t)) => match t.as_ref() {
                "string" => ConfigType::String,
                "int" => ConfigType::Int,
                "float" => ConfigType::Float,
                "bool" => ConfigType::Bool,
                "keyword" => ConfigType::Keyword,
                "enum" => match map_get(spec_map, "values") {
                    Some(Value::Vector(vals)) | Some(Value::List(vals)) => {
                        let mut allowed = Vec::new();
                        for v in vals.iter() {
                            match v {
                                Value::Keyword(k) => allowed.push(k.to_string()),
                                Value::Str(s) => allowed.push(s.to_string()),
                                other => {
                                    return field_err(
                                        &format!(":config :{name} :values"),
                                        format!("expected keywords, got {other}"),
                                    );
                                }
                            }
                        }
                        ConfigType::Enum(allowed)
                    }
                    _ => {
                        return field_err(
                            &format!(":config :{name}"),
                            ":enum requires :values [...]",
                        );
                    }
                },
                other => {
                    return field_err(
                        &format!(":config :{name} :type"),
                        format!("unknown type :{other}"),
                    );
                }
            },
            Some(other) => {
                return field_err(
                    &format!(":config :{name} :type"),
                    format!("expected a keyword, got {other}"),
                );
            }
            None => return field_err(&format!(":config :{name}"), "missing :type"),
        };
        let default = map_get(spec_map, "default").cloned();
        if let Some(d) = &default {
            ty.check(&name, d)?;
        }
        let doc = opt_string(spec_map, "doc")?;
        out.push(ConfigSpec {
            name,
            ty,
            default,
            doc,
        });
    }
    Ok(out)
}

/// The reserved niceties a well-formed `manifest.ld` starts with: an SPDX
/// comment and a docstring comment are *comments* — the reader skips them —
/// but this helper extracts the SPDX identifier for tooling that wants it
/// without reading the whole file.
pub fn spdx_from_header_comments(src: &str) -> Option<String> {
    src.lines()
        .take(8)
        .filter_map(|line| line.trim().strip_prefix(";"))
        .find_map(|comment| {
            comment
                .trim()
                .strip_prefix("SPDX-License-Identifier:")
                .map(|s| s.trim().to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
        ; SPDX-License-Identifier: MPL-2.0
        {:name "word-count"
         :version "0.1.0"
         :spdx "MPL-2.0"
         :tier :ayo
         :description "Counts words per note"
         :entry-point "src/main.ld"
         :capabilities [:notes/read :notes/write]
         :config {:min-words {:type :int :default 0 :doc "ignore shorter notes"}
                  :mode {:type :enum :values [:fast :careful] :default :fast}}
         :tests ["test/main.test.ld"]}
    "#;

    #[test]
    fn parses_a_full_manifest() {
        let m = PackageManifest::from_source(GOOD).unwrap();
        assert_eq!(m.name, "word-count");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.tier, Tier::Ayo);
        assert_eq!(m.entry_point, "src/main.ld");
        assert!(m.requested.allows(&Capability::NotesRead));
        assert!(m.requested.allows(&Capability::NotesWrite));
        assert!(!m.requested.allows(&Capability::AgentsRun));
        assert_eq!(m.config.len(), 2);
        assert_eq!(m.tests, vec!["test/main.test.ld".to_string()]);
        assert_eq!(spdx_from_header_comments(GOOD).as_deref(), Some("MPL-2.0"));
    }

    #[test]
    fn minimal_manifest_defaults() {
        let m = PackageManifest::from_source(
            r#"{:name "bare" :version "1.0.0" :entry-point "src/main.ld"}"#,
        )
        .unwrap();
        assert_eq!(m.tier, Tier::Ayo);
        assert_eq!(m.config.len(), 0);
        assert!(!m.requested.allows(&Capability::NotesRead));
    }

    #[test]
    fn rejects_unknown_capabilities() {
        let err = PackageManifest::from_source(
            r#"{:name "x-ray" :version "1.0.0" :entry-point "m.ld" :capabilities [:disk/write]}"#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("unknown capability"));
    }

    #[test]
    fn rejects_bad_names_and_versions() {
        assert!(PackageManifest::from_source(
            r#"{:name "Bad_Name" :version "1.0.0" :entry-point "m.ld"}"#
        )
        .is_err());
        assert!(
            PackageManifest::from_source(r#"{:name "ok" :version "1.0" :entry-point "m.ld"}"#)
                .is_err()
        );
        assert!(PackageManifest::from_source(
            r#"{:name "ok" :version "1.0.0" :entry-point "../escape.ld"}"#
        )
        .is_err());
    }

    #[test]
    fn config_resolution_validates() {
        let m = PackageManifest::from_source(GOOD).unwrap();
        // defaults fill
        let resolved = m.resolve_config(&[]).unwrap();
        assert_eq!(resolved.len(), 2);
        // override with correct type
        let resolved = m
            .resolve_config(&[("min-words".into(), Value::Int(10))])
            .unwrap();
        assert!(resolved
            .iter()
            .any(|(k, v)| k == "min-words" && matches!(v, Value::Int(10))));
        // wrong type rejected
        assert!(m
            .resolve_config(&[("min-words".into(), Value::str("ten"))])
            .is_err());
        // unknown key rejected
        assert!(m
            .resolve_config(&[("bogus".into(), Value::Int(1))])
            .is_err());
        // enum membership enforced
        assert!(m
            .resolve_config(&[("mode".into(), Value::kw("reckless"))])
            .is_err());
    }
}
