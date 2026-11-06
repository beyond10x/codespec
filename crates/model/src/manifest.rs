//! The adapter manifest `codespec-adapter/1`: what an adapter claims to handle, printed by
//! `<adapter> --describe` (`docs/conformance.md` § 5).
//!
//! One JSON object:
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-adapter/1` |
//! | `role` | string | `reader` or `writer` |
//! | `lang` | string | the source language, not empty |
//! | `version` | string | the adapter's version, not empty |
//! | `cir` | string | the `cir` format it reads or writes, `cir/1` |
//! | `features` | array of string | the claim: feature ids, or `<prefix>.*` |
//! | `not` | array of string, optional | subtracted from the claim: feature ids, or `<prefix>.*` |
//!
//! An entry is a feature id of the vocabulary, or a family wildcard: an id prefix ending in `.*`
//! (`stmt.*`, `op.*`), naming every id that starts with the text before the `*`. A wildcard that
//! names no id is an unknown family and refused; so is a `*` anywhere else, an id the vocabulary
//! does not have and an entry repeated within one list. `not` may subtract an id the claim does not
//! hold. The claim is `features` expanded, minus `not` expanded.
//!
//! [`manifest_from_json`] reads strictly: unknown fields, a field stated twice, another format, role
//! or `cir`, a reader claiming a `fact.` feature (facts are computed by `codespec pass`) and an
//! empty `lang` or `version` are refused as well. [`AdapterManifest::to_json`] is the canonical
//! encoding the in-repository adapters print: fields in the order above, two-space indentation, a
//! final newline, `not` left out when empty.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Feature, vocabulary};

/// The format of an adapter manifest.
pub const ADAPTER_FORMAT: &str = "codespec-adapter/1";

/// The roles an adapter manifest admits.
pub const ADAPTER_ROLES: &[&str] = &["reader", "writer"];

/// A `codespec-adapter/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterManifest {
    pub format: String,
    pub role: String,
    pub lang: String,
    pub version: String,
    pub cir: String,
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not: Vec<String>,
}

/// Why a document is not a `codespec-adapter/1` manifest.
#[derive(Debug)]
pub enum ManifestReadError {
    Json(serde_json::Error),
    Format(String),
    /// `role` is neither `reader` nor `writer`; holds what it is.
    Role(String),
    /// `cir` is not `cir/1`; holds what it is.
    Cir(String),
    /// The named field is empty.
    Empty(String),
    /// An entry names an id the vocabulary does not have.
    UnknownFeature(String),
    /// A wildcard names no id of the vocabulary.
    UnknownFamily(String),
    /// A `*` that is not the `*` of a trailing `.*` after a prefix.
    Pattern(String),
    /// An entry appears twice in one list.
    Repeated(String),
    /// A reader claims a fact: facts are computed by `codespec pass`, never read
    /// (`docs/conformance.md` § 2).
    ReaderFact(String),
}

impl std::fmt::Display for ManifestReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestReadError::Json(e) => write!(f, "not a {ADAPTER_FORMAT} manifest: {e}"),
            ManifestReadError::Format(found) => {
                write!(
                    f,
                    "format is {found:?}, this reader reads {ADAPTER_FORMAT:?}"
                )
            }
            ManifestReadError::Role(found) => {
                write!(f, "role is {found:?}, one of {ADAPTER_ROLES:?}")
            }
            ManifestReadError::Cir(found) => {
                write!(f, "cir is {found:?}, this reader knows {:?}", crate::FORMAT)
            }
            ManifestReadError::Empty(field) => write!(f, "`{field}` is empty"),
            ManifestReadError::UnknownFeature(id) => {
                write!(f, "unknown feature {id:?}: the vocabulary has no such id")
            }
            ManifestReadError::UnknownFamily(p) => {
                write!(f, "wildcard {p:?} names no feature: unknown family")
            }
            ManifestReadError::Pattern(p) => write!(
                f,
                "entry {p:?}: a `*` is allowed only as a trailing `.*` after a family prefix"
            ),
            ManifestReadError::Repeated(e) => write!(f, "entry {e:?} appears twice in one list"),
            ManifestReadError::ReaderFact(id) => write!(
                f,
                "a reader claims {id:?}: facts are computed by `codespec pass`, a writer feature only"
            ),
        }
    }
}

impl std::error::Error for ManifestReadError {}

/// The ids one list names against `kit`, refusing what the module doc refuses.
fn expand(entries: &[String], kit: &[Feature]) -> Result<BTreeSet<String>, ManifestReadError> {
    let mut seen = BTreeSet::new();
    let mut out = BTreeSet::new();
    for e in entries {
        if !seen.insert(e.as_str()) {
            return Err(ManifestReadError::Repeated(e.clone()));
        }
        match e.find('*') {
            None => {
                if !kit.iter().any(|f| &f.id == e) {
                    return Err(ManifestReadError::UnknownFeature(e.clone()));
                }
                out.insert(e.clone());
            }
            Some(at) => {
                let prefix = &e[..at];
                if at + 1 != e.len() || !prefix.ends_with('.') || prefix.len() < 2 {
                    return Err(ManifestReadError::Pattern(e.clone()));
                }
                let family: Vec<String> = kit
                    .iter()
                    .filter(|f| f.id.starts_with(prefix))
                    .map(|f| f.id.clone())
                    .collect();
                if family.is_empty() {
                    return Err(ManifestReadError::UnknownFamily(e.clone()));
                }
                out.extend(family);
            }
        }
    }
    Ok(out)
}

impl AdapterManifest {
    /// The claim expanded against `kit` (the features of a `kit.json` or the vocabulary): `features`
    /// minus `not`, every entry checked as the module doc says.
    pub fn claimed_in(&self, kit: &[Feature]) -> Result<BTreeSet<String>, ManifestReadError> {
        let claimed = expand(&self.features, kit)?;
        let not = expand(&self.not, kit)?;
        Ok(claimed.difference(&not).cloned().collect())
    }

    /// The claim expanded against [`vocabulary`]; a manifest from [`manifest_from_json`] always
    /// expands.
    pub fn claimed(&self) -> BTreeSet<String> {
        self.claimed_in(&vocabulary())
            .unwrap_or_else(|e| panic!("a manifest read strictly expands: {e}"))
    }

    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a manifest always serializes");
        s.push('\n');
        s
    }
}

/// Read a `codespec-adapter/1` manifest as the module doc defines it, expanding it against
/// [`vocabulary`].
pub fn manifest_from_json(bytes: &[u8]) -> Result<AdapterManifest, ManifestReadError> {
    let probe: Value = serde_json::from_slice(bytes).map_err(ManifestReadError::Json)?;
    let format = probe.get("format").and_then(Value::as_str).unwrap_or("");
    if format != ADAPTER_FORMAT {
        return Err(ManifestReadError::Format(format.to_string()));
    }
    let m: AdapterManifest = serde_json::from_slice(bytes).map_err(ManifestReadError::Json)?;
    if !ADAPTER_ROLES.contains(&m.role.as_str()) {
        return Err(ManifestReadError::Role(m.role));
    }
    if m.cir != crate::FORMAT {
        return Err(ManifestReadError::Cir(m.cir));
    }
    for (field, value) in [("lang", &m.lang), ("version", &m.version)] {
        if value.is_empty() {
            return Err(ManifestReadError::Empty(field.into()));
        }
    }
    let claimed = m.claimed_in(&vocabulary())?;
    if m.role == "reader"
        && let Some(id) = claimed.iter().find(|id| id.starts_with("fact."))
    {
        return Err(ManifestReadError::ReaderFact(id.clone()));
    }
    Ok(m)
}
