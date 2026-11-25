//! The badges at the top of `README.md`.
//!
//! Three are shields.io's own, from the repository: the latest release, the `check` workflow on
//! `main`, the licence. Three are `endpoint` badges that read a JSON file of this repository on
//! `main` (`website/data/badges/*.json`, the shields endpoint format), each a number counted in a
//! committed data file by this module and checked byte for byte like every generated file:
//!
//! | badge | counted in | links to |
//! |---|---|---|
//! | languages read | `website/data/support-matrix.json`: the languages whose read column is not `none` | the support matrix page |
//! | features verified | the same file: `verified` cells of every read and write column, of the cells that are not `none` or `n/a` | the support matrix page |
//! | definite or possible answers | `website/data/comparison.json`: the tools with `yes` on S3, of all tools | the S3 notes of the comparison page |
//!
//! The README holds the generated block between [`BEGIN`] and [`END`]. A badge for the reader
//! corpus KPIs of `story:corpus-kpi-ratchet` goes where [`SLOT`] stands, once that story has a
//! baseline to count: no badge shows a number nothing holds.

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value as Json;

use crate::Result;
use crate::comparison::{Comparison, Value};

/// The README, relative to the repository root.
pub const README: &str = "README.md";
/// The line before the generated block.
pub const BEGIN: &str = "<!-- badges: written by codespec-docs from website/data, do not edit -->";
/// The line after it.
pub const END: &str = "<!-- /badges -->";
/// The line inside the block where the reader KPI badge goes.
pub const SLOT: &str =
    "<!-- slot: reader KPI badge of story:corpus-kpi-ratchet, added with its baseline -->";

const SITE: &str = "https://beyond10x.github.io/codespec";
const REPO: &str = "https://github.com/beyond10x/codespec";
const RAW: &str = "https://raw.githubusercontent.com/beyond10x/codespec/main";

pub const LANGUAGES: &str = "website/data/badges/languages-read.json";
pub const FEATURES: &str = "website/data/badges/features-verified.json";
pub const ANSWERS: &str = "website/data/badges/answers-definite-possible.json";

/// A shields.io endpoint badge.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub schema_version: u32,
    pub label: String,
    pub message: String,
    pub color: String,
}

/// What the support matrix counts: languages with a reader, verified cells, and the cells that
/// could be verified.
#[derive(Debug, PartialEq, Eq)]
pub struct Support {
    pub languages_read: usize,
    pub verified: usize,
    pub cells: usize,
}

fn state(cell: &Json) -> &str {
    cell.as_str()
        .or_else(|| cell["state"].as_str())
        .unwrap_or("")
}

/// The counts of the support matrix document `doc`.
pub fn support(doc: &str) -> Result<Support> {
    let doc: Json = serde_json::from_str(doc).map_err(|e| format!("support matrix: {e}"))?;
    let languages = doc["languages"]
        .as_array()
        .ok_or("support matrix: no languages")?;
    let mut out = Support {
        languages_read: 0,
        verified: 0,
        cells: 0,
    };
    for lang in languages {
        let read = lang["read"]
            .as_object()
            .ok_or("support matrix: no read column")?;
        if read.values().any(|c| state(c) != "none") {
            out.languages_read += 1;
        }
        for key in ["read", "write"] {
            let column = lang[key].as_object().ok_or("support matrix: no column")?;
            for cell in column.values() {
                match state(cell) {
                    "none" | "n/a" => {}
                    "verified" => {
                        out.verified += 1;
                        out.cells += 1;
                    }
                    _ => out.cells += 1,
                }
            }
        }
    }
    if out.languages_read == 0 || out.cells == 0 {
        return Err("support matrix: no reader or no cell".to_owned());
    }
    Ok(out)
}

/// The number of tools with `yes` on dimension `S3`, and of all tools.
pub fn answers(c: &Comparison) -> Result<(usize, usize)> {
    if !c.dimensions.iter().any(|d| d.id == "S3") {
        return Err("comparison: no dimension S3".to_owned());
    }
    let yes = c
        .cells
        .iter()
        .filter(|x| x.dimension == "S3" && x.value == Value::Yes)
        .count();
    Ok((yes, c.tools.len()))
}

fn endpoint(label: &str, message: String) -> String {
    let badge = Endpoint {
        schema_version: 1,
        label: label.to_owned(),
        message,
        color: "blue".to_owned(),
    };
    let mut s = serde_json::to_string_pretty(&badge).expect("a badge always serializes");
    s.push('\n');
    s
}

/// The three endpoint files, by path, from the support matrix document and the comparison.
pub fn files(matrix: &str, c: &Comparison) -> Result<Vec<(&'static str, String)>> {
    let s = support(matrix)?;
    let (yes, tools) = answers(c)?;
    Ok(vec![
        (
            LANGUAGES,
            endpoint("languages read", s.languages_read.to_string()),
        ),
        (
            FEATURES,
            endpoint(
                "features verified",
                format!("{} of {} cells", s.verified, s.cells),
            ),
        ),
        (
            ANSWERS,
            endpoint(
                "definite or possible answers",
                format!("yes in {yes} of {tools} tools"),
            ),
        ),
    ])
}

/// `text` percent-encoded for the query of a URL.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn badge(alt: &str, image: &str, link: &str) -> String {
    format!("[![{alt}]({image})]({link})\n")
}

fn from_endpoint(alt: &str, file: &str, link: &str) -> String {
    badge(
        alt,
        &format!(
            "https://img.shields.io/endpoint?url={}",
            encode(&format!("{RAW}/{file}"))
        ),
        link,
    )
}

/// The block between the markers of the README, without them.
pub fn block() -> String {
    let mut out = String::new();
    out.push_str(&badge(
        "release",
        "https://img.shields.io/github/v/release/beyond10x/codespec",
        &format!("{REPO}/releases/latest"),
    ));
    out.push_str(&badge(
        "check on main",
        "https://img.shields.io/github/actions/workflow/status/beyond10x/codespec/check.yml?branch=main&label=check",
        &format!("{REPO}/actions/workflows/check.yml?query=branch%3Amain"),
    ));
    out.push_str(&badge(
        "licence",
        "https://img.shields.io/github/license/beyond10x/codespec",
        "LICENSE",
    ));
    out.push_str(&from_endpoint(
        "languages read",
        LANGUAGES,
        &format!("{SITE}/docs/reference/support"),
    ));
    out.push_str(&from_endpoint(
        "features verified",
        FEATURES,
        &format!("{SITE}/docs/reference/support"),
    ));
    out.push_str(&from_endpoint(
        "definite or possible answers",
        ANSWERS,
        &format!("{SITE}/docs/comparison#dim-s3"),
    ));
    out.push_str(SLOT);
    out.push('\n');
    out
}

/// `text` with the lines between its one [`BEGIN`] line and the next [`END`] line replaced by
/// `block`.
pub fn splice(text: &str, block: &str) -> Result<String> {
    let begin = format!("{BEGIN}\n");
    let end = format!("{END}\n");
    let (head, rest) = text
        .split_once(&begin)
        .ok_or_else(|| format!("{README}: no line `{BEGIN}`"))?;
    if rest.contains(&begin) {
        return Err(format!("{README}: more than one line `{BEGIN}`"));
    }
    let (_, tail) = rest
        .split_once(&end)
        .ok_or_else(|| format!("{README}: no line `{END}` after `{BEGIN}`"))?;
    Ok(format!("{head}{begin}{block}{end}{tail}"))
}

/// The README as it is in `root`, its badge block replaced by [`block`].
pub fn readme(root: &Path) -> Result<String> {
    let found = fs::read_to_string(root.join(README)).map_err(|e| format!("{README}: {e}"))?;
    splice(&found, &block())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATRIX: &str = r#"{"languages": [
        {"name": "Go", "read": {"a": "verified", "b": "refused", "c": "n/a"},
         "write": {"a": {"state": "verified"}, "b": "failing"}},
        {"name": "Java", "read": {"a": "verified", "b": "claimed", "c": "n/a"},
         "write": {"a": "none", "b": "none"}},
        {"name": "Evaluator", "read": {"a": "none", "b": "none", "c": "none"},
         "write": {"a": "verified", "b": "refused"}}
    ]}"#;

    #[test]
    fn the_support_counts_skip_none_and_not_applicable() {
        assert_eq!(
            support(MATRIX).unwrap(),
            Support {
                languages_read: 2,
                verified: 4,
                cells: 8
            }
        );
        assert!(support("{\"languages\": []}").is_err());
        assert!(support("[]").is_err());
    }

    #[test]
    fn the_count_of_this_tree_matches_its_matrix_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let doc = fs::read_to_string(root.join("website/data/support-matrix.json")).unwrap();
        let s = support(&doc).unwrap();
        assert!(s.languages_read >= 8, "{s:?}");
        assert!(s.verified > 0 && s.verified <= s.cells, "{s:?}");
    }

    #[test]
    fn an_endpoint_file_is_the_shields_format() {
        let text = endpoint("languages read", "8".to_owned());
        let json: Json = serde_json::from_str(&text).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["label"], "languages read");
        assert_eq!(json["message"], "8");
        assert!(json["color"].is_string());
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn the_endpoint_url_names_the_file_on_main_encoded() {
        let b = from_endpoint("x", LANGUAGES, "https://example.org/p");
        assert!(b.contains(
            "url=https%3A%2F%2Fraw.githubusercontent.com%2Fbeyond10x%2Fcodespec%2Fmain%2Fwebsite%2Fdata%2Fbadges%2Flanguages-read.json)"
        ));
        assert!(b.ends_with("](https://example.org/p)\n"));
    }

    #[test]
    fn the_block_links_each_badge_and_holds_the_slot() {
        let b = block();
        assert_eq!(b.lines().filter(|l| l.starts_with("[![")).count(), 6);
        assert!(b.lines().all(|l| l.starts_with("[![") || l == SLOT));
        assert!(b.contains("docs/comparison#dim-s3"));
        assert!(b.contains("docs/reference/support"));
    }

    #[test]
    fn splice_needs_both_markers_and_keeps_the_rest() {
        let text = format!("a\n{BEGIN}\nold\n{END}\nb\n");
        assert_eq!(
            splice(&text, "new\n").unwrap(),
            format!("a\n{BEGIN}\nnew\n{END}\nb\n")
        );
        assert!(splice("a\nb\n", "x\n").is_err());
        assert!(splice(&format!("{BEGIN}\nold\n"), "x\n").is_err());
        assert!(splice(&format!("{BEGIN}\n{END}\n{BEGIN}\n{END}\n"), "x\n").is_err());
    }
}
