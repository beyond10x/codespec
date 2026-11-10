//! The support matrix, `website/data/support-matrix.json` (`codespec-support-matrix/1`,
//! `schemas/support-matrix-1.schema.json`): per `cir/1` feature and per language, the state of
//! reading it and of writing it, for the page `/docs/reference/support`.
//!
//! It is built from three inputs and nothing written by hand: the feature list
//! `schemas/cir-1.features.json` (id, family, op, description; the rows, in its order), each
//! adapter's `--describe` manifest, expanded against the kit `conformance/kit.json` by
//! [`codespec_conformance::conform::claim`], and the adapter's conformance report under
//! `website/data/conformance/`. A report must have been run on the manifest it is shown with: its
//! adapter, version and claimed set equal the manifest's, or the build fails.
//!
//! | cell | when |
//! |---|---|
//! | `none` | the language has no adapter for the role |
//! | `n/a` | a reader cell of a `fact.*` feature: facts are computed by `codespec pass` |
//! | `verified` | claimed, and the report's verdict is `pass` |
//! | `claimed` | claimed, and no report, or the report has no case for it (`no_case`) |
//! | `failing` | claimed, and the report fails it for any other reason |
//! | `refused` | not claimed |
//!
//! A cell carries a `note` only where the report gives a reason: a failing cell names its case and
//! the first difference, or the step and reason it stopped at; a claimed cell without a case says
//! so; a refused cell whose case the report ran says how the adapter refused it, or that it passes
//! without a claim. A cell without a report entry for its reason has no note.
//!
//! A row's `case` is the first kit case, in name order, whose subject is the feature.

use std::collections::BTreeSet;
use std::path::Path;

use codespec_conformance::conform::{
    EVAL_RUNNER, EVAL_WRITER, FeatureResult, Reason, Report, Verdict, claim, read_kit,
};
use codespec_conformance::load_cases;
use serde_json::{Map, Value, json};

use crate::Result;

/// The generated matrix, relative to the repository root.
pub const DATA: &str = "website/data/support-matrix.json";

/// The format of the matrix.
pub const FORMAT: &str = "codespec-support-matrix/1";

/// The languages of the matrix, in column order: display name and the `lang` of its adapters.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("Go", "go"),
    ("Rust", "rust"),
    ("Java", "java"),
    ("Groovy", "groovy"),
    ("C", "c"),
    ("Evaluator", "cir"),
];

/// One adapter as the matrix sees it: its manifest's bytes, when one exists its report, and the
/// reasons the adapter gives for features it leaves out of its claim (by feature id), which the
/// note of such a cell carries.
pub struct Adapter {
    pub role: &'static str,
    pub lang: &'static str,
    pub manifest: Vec<u8>,
    pub report: Option<Report>,
    pub reasons: &'static [(&'static str, &'static str)],
}

/// The matrix for the feature list `features` (the bytes of `schemas/cir-1.features.json`), the
/// kit at `kit` and `adapters`; a language or role with no entry in `adapters` is `none`.
pub fn document(features: &[u8], kit: &Path, adapters: &[Adapter]) -> Result<String> {
    let vocabulary =
        codespec_model::features_from_json(features).map_err(|e| format!("features: {e}"))?;
    let kit_doc = read_kit(kit)?;
    let cases = load_cases(&kit.join("cases"))?;
    let mut rows = Vec::new();
    for f in &vocabulary.features {
        let case = cases
            .iter()
            .find(|c| c.subject() == f.id)
            .map(|c| Value::String(c.name.clone()))
            .unwrap_or(Value::Null);
        let mut row = Map::new();
        row.insert("id".into(), json!(f.id));
        row.insert("family".into(), json!(f.family));
        if let Some(op) = &f.op {
            row.insert("op".into(), json!(op));
        }
        row.insert("description".into(), json!(f.description));
        row.insert("case".into(), case);
        rows.push(Value::Object(row));
    }
    let mut languages = Vec::new();
    for (name, lang) in LANGUAGES {
        let mut column = Map::new();
        column.insert("name".into(), json!(name));
        for (role, key) in [("reader", "read"), ("writer", "write")] {
            let adapter = adapters.iter().find(|a| a.role == role && a.lang == *lang);
            let mut cells = Map::new();
            match adapter {
                None => {
                    for f in &vocabulary.features {
                        cells.insert(f.id.clone(), json!("none"));
                    }
                }
                Some(a) => {
                    let (manifest, claimed) = claim(&a.manifest, &kit_doc)
                        .map_err(|e| format!("codespec-{role}-{lang} --describe: {e}"))?;
                    if manifest.role != role || manifest.lang != *lang {
                        return Err(format!(
                            "codespec-{role}-{lang} --describe names {} {}",
                            manifest.role, manifest.lang
                        ));
                    }
                    if let Some(r) = &a.report {
                        check_report(r, role, lang, &manifest.version, &claimed)?;
                    }
                    for f in &vocabulary.features {
                        let entry = a
                            .report
                            .as_ref()
                            .and_then(|r| r.features.iter().find(|e| e.id == f.id));
                        let cell = if role == "reader" && f.family == "fact" {
                            ("n/a", None)
                        } else {
                            let (s, note) =
                                state(claimed.contains(&f.id), a.report.is_some(), entry, name);
                            let why = a.reasons.iter().find(|(id, _)| *id == f.id);
                            match (why, note) {
                                (Some((_, why)), Some(n)) if !claimed.contains(&f.id) => {
                                    (s, Some(format!("{n} {why}")))
                                }
                                (Some((_, why)), None) if !claimed.contains(&f.id) => {
                                    (s, Some(why.to_string()))
                                }
                                (_, note) => (s, note),
                            }
                        };
                        cells.insert(f.id.clone(), cell_value(cell));
                    }
                }
            }
            column.insert(key.into(), Value::Object(cells));
        }
        languages.push(Value::Object(column));
    }
    let doc = json!({
        "format": FORMAT,
        "cir": vocabulary.cir,
        "features": rows,
        "languages": languages,
    });
    let mut s = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    s.push('\n');
    Ok(s)
}

/// Refuse a report that was not run on this manifest.
fn check_report(
    r: &Report,
    role: &str,
    lang: &str,
    version: &str,
    claimed: &BTreeSet<String>,
) -> Result<()> {
    let path = format!("website/data/conformance/{role}-{lang}.json");
    if r.adapter.role != role || r.adapter.lang != lang || r.adapter.version != version {
        return Err(format!(
            "{path} is for {} {} {}, the manifest is {role} {lang} {version}; run `task docs-generate`",
            r.adapter.role, r.adapter.lang, r.adapter.version
        ));
    }
    let in_report: BTreeSet<String> = r
        .features
        .iter()
        .filter(|e| e.claimed)
        .map(|e| e.id.clone())
        .collect();
    if &in_report != claimed {
        let more: Vec<_> = claimed.difference(&in_report).collect();
        let fewer: Vec<_> = in_report.difference(claimed).collect();
        return Err(format!(
            "{path} claims other features than codespec-{role}-{lang} --describe (manifest only: \
             {more:?}; report only: {fewer:?}); run `task docs-generate`"
        ));
    }
    Ok(())
}

/// The state of one applicable cell and its note.
fn state(
    claimed: bool,
    has_report: bool,
    entry: Option<&FeatureResult>,
    language: &str,
) -> (&'static str, Option<String>) {
    let Some(e) = entry.filter(|_| has_report) else {
        return (if claimed { "claimed" } else { "refused" }, None);
    };
    match (claimed, e.verdict) {
        (true, Verdict::Pass) => ("verified", None),
        (true, _) if e.reason == Some(Reason::NoCase) => (
            "claimed",
            Some(format!(
                "Claimed, but no {language} conformance case covers it, so the report cannot verify it."
            )),
        ),
        (true, _) => ("failing", Some(failure(e))),
        (false, Verdict::Pass) => (
            "refused",
            Some(format!(
                "Not claimed, though its case {} passes.",
                e.cases.join(", ")
            )),
        ),
        (false, Verdict::Refused) => ("refused", Some(refusal(e))),
        (false, _) => ("refused", None),
    }
}

/// The note of a failing cell: the case, then the first difference or where and why it stopped.
fn failure(e: &FeatureResult) -> String {
    let case = e.case.as_deref().unwrap_or("?");
    if let Some(d) = &e.first_difference {
        return format!(
            "Case {case}: line {} expected {:?}, got {:?}.",
            d.line, d.expected, d.got
        );
    }
    let mut s = format!("Case {case}");
    if let Some(step) = e.step {
        s.push_str(&format!(", step {step}"));
    }
    if let Some(r) = e.reason {
        s.push_str(&format!(": {r}"));
    }
    if let Some(d) = first_line(e.detail.as_deref()) {
        s.push_str(&format!(": {d}"));
    }
    s.push('.');
    s
}

/// The note of a refused cell whose case was run: how the adapter refused it, with the adapter's
/// own message where the report keeps one.
fn refusal(e: &FeatureResult) -> String {
    let case = e.case.as_deref().unwrap_or("?");
    let (how, message) = match e.reason {
        Some(Reason::SubjectMissing) => ("its module does not use the feature".to_string(), None),
        Some(Reason::ForeignStatement) => (
            "the reader keeps part of it as a foreign statement".to_string(),
            None,
        ),
        Some(Reason::ReadFailed) => ("the reader refuses it".to_string(), e.detail.as_deref()),
        Some(Reason::Refused) if e.runner.as_deref() == Some(EVAL_RUNNER) => (
            "the Go writer refuses it and the evaluator stops in it".to_string(),
            e.detail.as_deref(),
        ),
        Some(Reason::Refused) if e.runner.as_deref() == Some(EVAL_WRITER) => (
            "the Go writer and the evaluator's writer refuse it".to_string(),
            e.detail.as_deref(),
        ),
        Some(Reason::Refused) => ("the writer refuses it".to_string(), e.detail.as_deref()),
        Some(r) => (r.to_string(), e.detail.as_deref()),
        None => ("refused".to_string(), None),
    };
    match first_line(message) {
        Some(d) => format!("Case {case}: {how}: {d}."),
        None => format!("Case {case}: {how}."),
    }
}

/// The first non-empty line of a detail, without a final full stop.
fn first_line(detail: Option<&str>) -> Option<&str> {
    detail
        .and_then(|d| d.lines().map(str::trim).find(|l| !l.is_empty()))
        .map(|l| l.trim_end_matches('.'))
}

fn cell_value((state, note): (&str, Option<String>)) -> Value {
    match note {
        None => json!(state),
        Some(n) => json!({"state": state, "note": n}),
    }
}

/// A column's name (`Go read`) and its count per state.
pub type Column = (String, Vec<(String, usize)>);

/// The counts of every column, in column order, by state in legend order: what the page's bars
/// show.
pub fn totals(document: &str) -> Result<Vec<Column>> {
    let doc: Value = serde_json::from_str(document).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for lang in doc["languages"].as_array().into_iter().flatten() {
        for key in ["read", "write"] {
            let mut counts: Vec<(String, usize)> =
                ["verified", "claimed", "failing", "refused", "n/a", "none"]
                    .iter()
                    .map(|s| (s.to_string(), 0))
                    .collect();
            for cell in lang[key].as_object().into_iter().flat_map(|m| m.values()) {
                let s = cell
                    .as_str()
                    .or_else(|| cell["state"].as_str())
                    .unwrap_or("");
                if let Some(c) = counts.iter_mut().find(|(n, _)| n == s) {
                    c.1 += 1;
                }
            }
            out.push((
                format!("{} {key}", lang["name"].as_str().unwrap_or("?")),
                counts,
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespec_conformance::conform::{AdapterRef, KitRef, Summary};
    use std::fs;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn manifest(role: &str, lang: &str, features: &[&str]) -> Vec<u8> {
        let list: Vec<String> = features.iter().map(|f| format!("{f:?}")).collect();
        format!(
            "{{\n  \"format\": \"codespec-adapter/1\",\n  \"role\": \"{role}\",\n  \"lang\": \"{lang}\",\n  \"version\": \"9.9.9\",\n  \"cir\": \"cir/1\",\n  \"features\": [{}]\n}}\n",
            list.join(", ")
        )
        .into_bytes()
    }

    fn entry(id: &str, claimed: bool, verdict: Verdict, reason: Option<Reason>) -> FeatureResult {
        FeatureResult {
            id: id.into(),
            claimed,
            verdict,
            cases: vec!["some-case".into()],
            case: (verdict != Verdict::Pass).then(|| "some-case".into()),
            step: None,
            reason,
            first_difference: (reason == Some(Reason::WrongOutput)).then(|| {
                codespec_conformance::conform::FirstDifference {
                    line: 2,
                    expected: "ok 1".into(),
                    got: "ok 2".into(),
                }
            }),
            detail: (reason == Some(Reason::SubjectMissing))
                .then(|| "the module does not use op.add outside a foreign statement".into()),
            runner: None,
        }
    }

    fn report(role: &str, lang: &str, features: Vec<FeatureResult>) -> Report {
        Report {
            format: "codespec-conformance-report/1".into(),
            adapter: AdapterRef {
                role: role.into(),
                lang: lang.into(),
                version: "9.9.9".into(),
                manifest_sha256: "0".repeat(64),
            },
            kit: KitRef {
                version: "9.9.9".into(),
                sha256: "0".repeat(64),
            },
            corpus: None,
            codespec: "9.9.9".into(),
            features,
            boundary: Vec::new(),
            summary: Summary::default(),
            conforms: false,
        }
    }

    fn cell<'a>(doc: &'a Value, lang: &str, key: &str, id: &str) -> &'a Value {
        let l = doc["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["name"] == lang)
            .unwrap();
        &l[key][id]
    }

    #[test]
    fn a_matrix_from_fixture_manifests_and_reports_reaches_every_state() {
        let root = root();
        let features = fs::read(root.join("schemas/cir-1.features.json")).unwrap();
        let kit = root.join("conformance");
        // A Go reader with a report: op.add verified, op.sub failing on a wrong output,
        // stmt.send claimed without a case, op.mul refused with a reason, op.div passing unclaimed.
        let go_reader = Adapter {
            role: "reader",
            lang: "go",
            manifest: manifest("reader", "go", &["op.add", "op.sub", "stmt.send"]),
            report: Some(report(
                "reader",
                "go",
                vec![
                    entry("op.add", true, Verdict::Pass, None),
                    entry("op.sub", true, Verdict::Fail, Some(Reason::WrongOutput)),
                    entry("stmt.send", true, Verdict::Fail, Some(Reason::NoCase)),
                    entry(
                        "op.mul",
                        false,
                        Verdict::Refused,
                        Some(Reason::SubjectMissing),
                    ),
                    entry("op.div", false, Verdict::Pass, None),
                ],
            )),
            reasons: &[("op.div", "A fixture reason it is left out.")],
        };
        // A Rust writer with a manifest and no report: every claim is `claimed`.
        let rust_writer = Adapter {
            role: "writer",
            lang: "rust",
            manifest: manifest("writer", "rust", &["op.add", "fact.fallible"]),
            report: None,
            reasons: &[],
        };
        // A Java reader and no Java writer: the write column is `none` for every row.
        let java_reader = Adapter {
            role: "reader",
            lang: "java",
            manifest: manifest("reader", "java", &["op.add"]),
            report: None,
            reasons: &[],
        };
        let text = document(&features, &kit, &[go_reader, rust_writer, java_reader]).unwrap();
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(doc["format"], FORMAT);
        assert_eq!(doc["features"][0]["id"], "assign.add");
        assert_eq!(doc["features"][0]["case"], "assign-add");

        assert_eq!(cell(&doc, "Go", "read", "op.add"), "verified");
        assert_eq!(cell(&doc, "Go", "read", "op.sub")["state"], "failing");
        assert_eq!(
            cell(&doc, "Go", "read", "op.sub")["note"],
            "Case some-case: line 2 expected \"ok 1\", got \"ok 2\"."
        );
        assert_eq!(cell(&doc, "Go", "read", "stmt.send")["state"], "claimed");
        assert_eq!(cell(&doc, "Go", "read", "op.mul")["state"], "refused");
        assert_eq!(
            cell(&doc, "Go", "read", "op.mul")["note"],
            "Case some-case: its module does not use the feature."
        );
        assert_eq!(cell(&doc, "Go", "read", "op.div")["state"], "refused");
        assert_eq!(
            cell(&doc, "Go", "read", "op.div")["note"],
            "Not claimed, though its case some-case passes. A fixture reason it is left out."
        );
        assert_eq!(cell(&doc, "Go", "read", "op.rem"), "refused");
        assert_eq!(cell(&doc, "Go", "read", "fact.fallible"), "n/a");
        assert_eq!(cell(&doc, "Rust", "write", "op.add"), "claimed");
        assert_eq!(cell(&doc, "Rust", "write", "fact.fallible"), "claimed");
        assert_eq!(cell(&doc, "Rust", "write", "op.sub"), "refused");
        assert_eq!(cell(&doc, "Rust", "read", "op.add"), "none");
        assert_eq!(cell(&doc, "Java", "read", "op.add"), "claimed");
        let ids: Vec<&str> = doc["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        for id in &ids {
            assert_eq!(cell(&doc, "Java", "write", id), "none", "{id}");
            assert_eq!(cell(&doc, "Groovy", "read", id), "none", "{id}");
        }

        let totals = totals(&text).unwrap();
        let go_read = &totals.iter().find(|(n, _)| n == "Go read").unwrap().1;
        for (s, n) in go_read {
            if s != "none" {
                assert!(*n > 0, "Go read has no {s} cell");
            }
        }
        let java_write = &totals.iter().find(|(n, _)| n == "Java write").unwrap().1;
        assert_eq!(java_write.last().unwrap(), &("none".to_string(), ids.len()));
    }

    #[test]
    fn a_report_run_on_another_claim_is_refused() {
        let root = root();
        let features = fs::read(root.join("schemas/cir-1.features.json")).unwrap();
        let stale = Adapter {
            role: "writer",
            lang: "go",
            manifest: manifest("writer", "go", &["op.add", "op.sub"]),
            report: Some(report(
                "writer",
                "go",
                vec![entry("op.add", true, Verdict::Pass, None)],
            )),
            reasons: &[],
        };
        let e = document(&features, &root.join("conformance"), &[stale]).unwrap_err();
        assert!(e.contains("manifest only: [\"op.sub\"]"), "{e}");
    }

    /// A reader's case the Go writer refuses: the note says whether the evaluator's writer refused
    /// the module or the evaluator ran it and stopped, with the evaluator's own message.
    #[test]
    fn a_refusal_judged_by_the_evaluator_says_whether_it_ran_and_stopped_or_was_not_run() {
        let judged = |runner: &str, detail: &str| {
            let mut e = entry("decl.union", false, Verdict::Refused, Some(Reason::Refused));
            e.step = Some(4);
            e.runner = Some(runner.into());
            e.detail = Some(detail.into());
            state(false, true, Some(&e), "Rust").1.unwrap()
        };
        assert_eq!(
            judged(
                "codespec-run-cir",
                "codespec-run-cir: stopped at m.F#0 (case.rs:3): unsupported: a verb"
            ),
            "Case some-case: the Go writer refuses it and the evaluator stops in it: \
             codespec-run-cir: stopped at m.F#0 (case.rs:3): unsupported: a verb."
        );
        assert_eq!(
            judged(
                "codespec-write-cir",
                "codespec-write-cir: refused F (case.rs:3): foreign rust statement: kept"
            ),
            "Case some-case: the Go writer and the evaluator's writer refuse it: \
             codespec-write-cir: refused F (case.rs:3): foreign rust statement: kept."
        );
    }
}
