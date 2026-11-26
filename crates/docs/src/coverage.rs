//! | `r    let text = fs::read_to_string(root.join(FLOORS)).map_err(|e| format!("{FLOORS}: {e}"))?;aders[].floor` | the reader's verified count; it may not drop below it |Reader coverage of the conformance kit, `codespec-docs reader-coverage` (`task
//! reader-coverage`, a step of `task check`).
//!
//! It reads the checked-in support matrix ([`support::DATA`]) and the floor file [`FLOORS`]
//! (`codespec-reader-floors/1`) and prints one line per reader of the floor file:
//!
//! ```text
//! rust 113/122 92.6%, 113/118 reachable
//! ```
//!
//! the reader's `verified` cells of the features a reader can claim (every cell of its column that
//! is not `n/a`: `fact.*` is computed by `codespec pass`), and of those the features its language
//! has a source form for: the applicable ones less the reader's `no_source_form` list. It prints
//! every line, then fails naming each reader whose verified count is not its `floor`: below it, the
//! reader lost a feature; above it, the floor is raised to the count in the same change.
//!
//! | field | meaning |
//! |---|---|
//! | `format` | always `codespec-reader-floors/1` |
//! | `readers[].lang` | the `lang` of a reader of the matrix ([`support::LANGUAGES`]) |
//! | `readers[].floor` | the reader's verified count, which may not drop |
//! | `readers[].no_source_form[]` | `{id, why}`: a feature its language has no source form for |
//!
//! The file is refused when an entry of `no_source_form` names a feature the matrix does not
//! have, one not applicable to a reader, one listed twice or without a reason, or one the reader
//! claims (its cell is `verified`, `claimed` or `failing`) or whose case passes unclaimed (its
//! refused cell says so): a feature a reader reads has a source form. An entry is for a feature
//! the language has no source form for, never for a limit of the reader. [`sourced`] also refuses
//! one a kit case holds a source of in the reader's language.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::{Result, support};

/// The floor file, relative to the repository root.
pub const FLOORS: &str = "conformance/reader-floors.json";

/// The format of the floor file.
pub const FORMAT: &str = "codespec-reader-floors/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Floors {
    format: String,
    readers: Vec<Reader>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reader {
    lang: String,
    floor: usize,
    no_source_form: Vec<Unreachable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unreachable {
    id: String,
    why: String,
}

/// The printed lines, one per reader of the floor file, and one message per reader whose count is
/// not its floor.
#[derive(Debug)]
pub struct Coverage {
    pub lines: Vec<String>,
    pub off_floor: Vec<String>,
}

/// The state of a matrix cell: a string, or the `state` of an object with a note.
fn state(cell: &Value) -> &str {
    cell.as_str()
        .or_else(|| cell["state"].as_str())
        .unwrap_or("")
}

/// The coverage of each reader of `floors` (the floor file's text) in `matrix` (the support
/// matrix's text).
pub fn coverage(matrix: &str, floors: &str) -> Result<Coverage> {
    let doc: Value = serde_json::from_str(matrix).map_err(|e| format!("{}: {e}", support::DATA))?;
    let floors: Floors = serde_json::from_str(floors).map_err(|e| format!("{FLOORS}: {e}"))?;
    if floors.format != FORMAT {
        return Err(format!(
            "{FLOORS}: format {:?}, expected {FORMAT:?}",
            floors.format
        ));
    }
    let mut out = Coverage {
        lines: Vec::new(),
        off_floor: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for r in &floors.readers {
        if !seen.insert(r.lang.as_str()) {
            return Err(format!("{FLOORS}: reader {} is listed twice", r.lang));
        }
        let name = support::LANGUAGES
            .iter()
            .find(|(_, lang)| *lang == r.lang)
            .map(|(name, _)| *name)
            .ok_or_else(|| format!("{FLOORS}: {} is no language of the matrix", r.lang))?;
        let cells = doc["languages"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|l| l["name"] == name)
            .and_then(|l| l["read"].as_object())
            .ok_or_else(|| format!("{}: no read column for {name}", support::DATA))?;
        if cells.values().all(|c| state(c) == "none") {
            return Err(format!("{FLOORS}: {} has no reader in the matrix", r.lang));
        }
        let applicable = cells.values().filter(|c| state(c) != "n/a").count();
        let verified = cells.values().filter(|c| state(c) == "verified").count();
        let mut listed = BTreeSet::new();
        for u in &r.no_source_form {
            let at = format!("{FLOORS}: {} no_source_form {}", r.lang, u.id);
            if !listed.insert(u.id.as_str()) {
                return Err(format!("{at}: listed twice"));
            }
            if u.why.trim().is_empty() {
                return Err(format!("{at}: no reason given"));
            }
            match cells.get(&u.id).map(state) {
                None => return Err(format!("{at}: no feature of the matrix")),
                Some("n/a") => return Err(format!("{at}: not applicable to a reader")),
                Some("refused") if unclaimed_pass(&cells[&u.id]) => {
                    return Err(format!(
                        "{at}: its case passes, so the reader reads it and it has a source form"
                    ));
                }
                Some("refused") => {}
                Some(s) => {
                    return Err(format!(
                        "{at}: the reader claims it (its cell is {s}), so it has a source form"
                    ));
                }
            }
        }
        let reachable = applicable - listed.len();
        out.lines.push(format!(
            "{} {verified}/{applicable} {:.1}%, {verified}/{reachable} reachable",
            r.lang,
            percent(verified, applicable)
        ));
        if verified < r.floor {
            out.off_floor.push(format!(
                "{} verified {verified}, below its floor {}",
                r.lang, r.floor
            ));
        } else if verified > r.floor {
            out.off_floor.push(format!(
                "{} verified {verified}, above its floor {}: raise its floor to {verified} in {FLOORS}",
                r.lang, r.floor
            ));
        }
    }
    Ok(out)
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 * 100.0 / whole as f64
    }
}

/// Whether a refused cell's note says its case passes: the reader reads the feature unclaimed.
fn unclaimed_pass(cell: &Value) -> bool {
    cell["note"]
        .as_str()
        .is_some_and(|n| n.starts_with(support::UNCLAIMED_PASS))
}

/// Refuse a `no_source_form` entry of `floors` (the floor file's text) whose feature is the subject
/// of a case of the kit at `kit` that holds a source in the reader's language
/// (`cases/<case>/<lang>`): someone wrote that source form. Go is not checked this way: every case
/// holds its Go reference `case.go`, also where the module is written by hand because the Go
/// reader gives another feature for that source.
///
/// [`step`] runs it after [`run`], and the test
/// `the_checked_in_floor_file_lists_no_feature_a_case_holds_a_source_of` on the checked-in files.
pub fn sourced(kit: &Path, floors: &str) -> Result<()> {
    let floors: Floors = serde_json::from_str(floors).map_err(|e| format!("{FLOORS}: {e}"))?;
    let cases = codespec_conformance::load_cases(&kit.join("cases"))?;
    for r in floors.readers.iter().filter(|r| r.lang != "go") {
        for u in &r.no_source_form {
            let dir =
                |c: &codespec_conformance::Case| kit.join("cases").join(&c.name).join(&r.lang);
            if let Some(c) = cases
                .iter()
                .find(|c| c.subject() == u.id && dir(c).is_dir())
            {
                return Err(format!(
                    "{FLOORS}: {} no_source_form {}: case {} holds a {} source of it, so it has a source form",
                    r.lang, u.id, c.name, r.lang
                ));
            }
        }
    }
    Ok(())
}

/// Print each reader's line from the matrix; fail naming every reader whose count is not its
/// floor.
pub fn run(root: &Path) -> Result<()> {
    let read = |rel: &str| fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"));
    let c = coverage(&read(support::DATA)?, &read(FLOORS)?)?;
    for line in &c.lines {
        println!("{line}");
    }
    if c.off_floor.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "reader coverage is not at its floor in {FLOORS}: {}",
            c.off_floor.join("; ")
        ))
    }
}

/// `codespec-docs reader-coverage`: [`run`] on the matrix, then [`sourced`] on the kit
/// `conformance/`.
pub fn step(root: &Path) -> Result<()> {
    run(root)?;
    let text = fs::read_to_string(root.join(FLOORS)).map_err(|e| format!("{FLOORS}: {e}"))?;
    sourced(&root.join("conformance"), &text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn checked_in(rel: &str) -> String {
        fs::read_to_string(root().join(rel)).unwrap()
    }

    /// The checked-in matrix with the first `verified` read cell of `name` set to `failing`.
    fn lowered(name: &str) -> String {
        let mut doc: Value = serde_json::from_str(&checked_in(support::DATA)).unwrap();
        let column = doc["languages"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|l| l["name"] == name)
            .unwrap();
        let cell = column["read"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .find(|c| state(c) == "verified")
            .unwrap();
        *cell = Value::String("failing".into());
        serde_json::to_string(&doc).unwrap()
    }

    fn floors_with(lang: &str, floor: usize, entries: &[(&str, &str)]) -> String {
        let list: Vec<Value> = entries
            .iter()
            .map(|(id, why)| serde_json::json!({"id": id, "why": why}))
            .collect();
        serde_json::json!({
            "format": FORMAT,
            "readers": [{"lang": lang, "floor": floor, "no_source_form": list}],
        })
        .to_string()
    }

    #[test]
    fn the_checked_in_floors_hold_the_four_readers_and_the_matrix_meets_them() {
        let floors: Floors = serde_json::from_str(&checked_in(FLOORS)).unwrap();
        let langs: Vec<&str> = floors.readers.iter().map(|r| r.lang.as_str()).collect();
        assert_eq!(langs, ["go", "rust", "java", "c"]);
        let c = coverage(&checked_in(support::DATA), &checked_in(FLOORS)).unwrap();
        assert_eq!(c.lines.len(), 4);
        assert_eq!(c.off_floor, Vec::<String>::new());
    }

    #[test]
    fn a_matrix_with_one_reader_count_lowered_fails_naming_the_reader_floor_and_count() {
        let matrix = checked_in(support::DATA);
        let floors = checked_in(FLOORS);
        let before = coverage(&matrix, &floors).unwrap();
        let rust_floor = serde_json::from_str::<Floors>(&floors)
            .unwrap()
            .readers
            .iter()
            .find(|r| r.lang == "rust")
            .unwrap()
            .floor;

        let dir = root().join(format!(
            "target/codespec-docs-test/reader-coverage-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("website/data")).unwrap();
        fs::create_dir_all(dir.join("conformance")).unwrap();
        fs::write(dir.join(support::DATA), lowered("Rust")).unwrap();
        fs::write(dir.join(FLOORS), &floors).unwrap();
        let e = run(&dir).unwrap_err();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            e,
            format!(
                "reader coverage is not at its floor in {FLOORS}: rust verified {}, below its floor {rust_floor}",
                rust_floor - 1
            )
        );

        // It still prints every line, the lowered one with one verified fewer.
        let after = coverage(&lowered("Rust"), &floors).unwrap();
        assert_eq!(after.lines.len(), before.lines.len());
        assert!(
            after.lines[1].starts_with(&format!("rust {}/", rust_floor - 1)),
            "{}",
            after.lines[1]
        );
    }

    #[test]
    fn a_line_gives_verified_of_the_applicable_and_of_the_reachable_features() {
        let matrix = checked_in(support::DATA);
        let doc: Value = serde_json::from_str(&matrix).unwrap();
        let features = doc["features"].as_array().unwrap();
        let applicable = features.iter().filter(|f| f["family"] != "fact").count();
        let rust = doc["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == "Rust")
            .unwrap();
        let verified = rust["read"]
            .as_object()
            .unwrap()
            .values()
            .filter(|c| state(c) == "verified")
            .count();
        let refused = rust["read"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, c)| state(c) == "refused")
            .map(|(id, _)| id.clone())
            .unwrap();
        let floors = floors_with("rust", 0, &[(&refused, "a fixture reason")]);
        let c = coverage(&matrix, &floors).unwrap();
        assert_eq!(
            c.lines,
            [format!(
                "rust {verified}/{applicable} {:.1}%, {verified}/{} reachable",
                percent(verified, applicable),
                applicable - 1
            )]
        );
    }

    #[test]
    fn a_floor_file_listing_a_feature_the_reader_claims_is_refused() {
        let matrix = checked_in(support::DATA);
        let doc: Value = serde_json::from_str(&matrix).unwrap();
        let go = doc["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == "Go")
            .unwrap();
        let claimed = go["read"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, c)| state(c) == "verified")
            .map(|(id, _)| id.clone())
            .unwrap();
        let e = coverage(&matrix, &floors_with("go", 0, &[(&claimed, "why")])).unwrap_err();
        assert_eq!(
            e,
            format!(
                "{FLOORS}: go no_source_form {claimed}: the reader claims it (its cell is verified), so it has a source form"
            )
        );
    }

    #[test]
    fn a_floor_file_with_an_unknown_fact_twice_listed_or_unexplained_entry_is_refused() {
        let matrix = checked_in(support::DATA);
        for (entries, says) in [
            (&[("op.nothing", "why")][..], "no feature of the matrix"),
            (
                &[("fact.fallible", "why")][..],
                "not applicable to a reader",
            ),
            (
                &[("decl.union", "why"), ("decl.union", "why")][..],
                "listed twice",
            ),
            (&[("decl.union", " ")][..], "no reason given"),
        ] {
            let e = coverage(&matrix, &floors_with("go", 0, entries)).unwrap_err();
            assert!(e.ends_with(says), "{e}");
        }
        let e = coverage(&matrix, &floors_with("cobol", 0, &[])).unwrap_err();
        assert!(e.ends_with("cobol is no language of the matrix"), "{e}");
        let e = coverage(&matrix, &floors_with("cir", 0, &[])).unwrap_err();
        assert!(e.ends_with("cir has no reader in the matrix"), "{e}");
    }

    /// The language support page states the applicable count from the matrix, not a number
    /// written by hand.
    #[test]
    fn the_support_page_takes_the_applicable_count_from_the_data() {
        let page = checked_in("website/docs/reference/support.mdx");
        let flat = page.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains(
                "a reader that handles all {matrix.features.filter((f) => f.family !== 'fact').length} of them reaches 100%"
            ),
            "support.mdx does not take the applicable count from the matrix"
        );
    }

    /// A count above its floor fails too, naming the floor to write, so each floor stays equal to
    /// its reader's count.
    #[test]
    fn a_count_above_its_floor_names_the_floor_to_write() {
        let matrix = with_cell("Go", "decl.union", Value::String("verified".into()));
        let c = coverage(&matrix, &floors_with("go", 116, &[])).unwrap();
        assert_eq!(
            c.off_floor,
            [format!(
                "go verified 117, above its floor 116: raise its floor to 117 in {FLOORS}"
            )]
        );
    }

    /// No entry of the checked-in floor file names a feature a kit case holds a source of in the
    /// reader's language.
    #[test]
    fn the_checked_in_floor_file_lists_no_feature_a_case_holds_a_source_of() {
        sourced(&root().join("conformance"), &checked_in(FLOORS)).unwrap();
    }

    #[test]
    fn a_floor_file_listing_a_feature_a_case_holds_a_source_of_is_refused() {
        let kit = root().join("conformance");
        let e = sourced(&kit, &floors_with("java", 0, &[("decl.variadic", "why")])).unwrap_err();
        assert_eq!(
            e,
            format!(
                "{FLOORS}: java no_source_form decl.variadic: case decl-variadic-local holds a java source of it, so it has a source form"
            )
        );
        // Go is not checked against its cases: every case holds a Go reference.
        sourced(&kit, &floors_with("go", 0, &[("decl.union", "why")])).unwrap();
    }

    /// The checked-in matrix with the read cell `id` of `name` replaced by `cell`.
    fn with_cell(name: &str, id: &str, cell: Value) -> String {
        let mut doc: Value = serde_json::from_str(&checked_in(support::DATA)).unwrap();
        let column = doc["languages"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|l| l["name"] == name)
            .unwrap();
        column["read"][id] = cell;
        serde_json::to_string(&doc).unwrap()
    }

    /// `run` on a tree holding `matrix` and the checked-in floor file.
    fn run_on(tag: &str, matrix: &str) -> Result<()> {
        let dir = root().join(format!(
            "target/codespec-docs-test/reader-coverage-adv-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("website/data")).unwrap();
        fs::create_dir_all(dir.join("conformance")).unwrap();
        fs::write(dir.join(support::DATA), matrix).unwrap();
        fs::write(dir.join(FLOORS), checked_in(FLOORS)).unwrap();
        let r = run(&dir);
        fs::remove_dir_all(&dir).unwrap();
        r
    }

    /// AGENTS.md § Invariants: "A reader that reads more raises its floor in the same change", and
    /// the story: each floor equals its reader's count. A matrix where Rust verifies one feature
    /// more than its floor passes the step, so the floor lags, and a later drop back to the old
    /// count is not caught.
    #[test]
    fn adv_a_count_above_its_floor_fails_so_the_floor_is_raised_with_it() {
        let doc: Value = serde_json::from_str(&checked_in(support::DATA)).unwrap();
        let floors: Floors = serde_json::from_str(&checked_in(FLOORS)).unwrap();
        let rust = floors.readers.iter().find(|r| r.lang == "rust").unwrap();
        let unlisted: Vec<&str> = rust.no_source_form.iter().map(|u| u.id.as_str()).collect();
        let id = doc["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == "Rust")
            .unwrap()["read"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(id, c)| state(c) == "refused" && !unlisted.contains(&id.as_str()))
            .map(|(id, _)| id.clone())
            .unwrap();
        let raised = with_cell("Rust", &id, Value::String("verified".into()));
        assert!(
            run_on("raised", &raised).is_err(),
            "rust verifies {} with its floor at {}: the step passes and the floor is not raised",
            rust.floor + 1,
            rust.floor
        );
    }

    /// A refused cell whose case passes ("Not claimed, though its case ... passes.", the state
    /// Groovy's `stmt.foreign` has today) is a feature the reader reads: it has a source form, and
    /// the floor file may not list it as having none.
    #[test]
    fn adv_a_floor_file_listing_a_feature_whose_case_passes_unclaimed_is_refused() {
        let matrix = with_cell(
            "Rust",
            "stmt.defer",
            serde_json::json!({
                "state": "refused",
                "note": "Not claimed, though its case stmt-defer passes."
            }),
        );
        let r = coverage(&matrix, &checked_in(FLOORS));
        assert!(
            r.is_err(),
            "rust lists stmt.defer as without a source form while its case passes: {:?}",
            r.map(|c| c.lines)
        );
    }
}
