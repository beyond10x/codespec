//! The sample `codespec conform` run in the adapter guide, written from the Go writer's
//! conformance report so the page never shows a version or count the report does not hold.
//!
//! The guide is hand-written except for the block between [`BEGIN`] and [`END`]: the command, the
//! summary line `codespec conform` prints for the report (`summary_line`), the line it prints for
//! the written file and the report's `summary` object as `jq -c .summary` prints it.

use std::fs;
use std::path::Path;

use codespec_conformance::conform::{Report, summary_line};

use crate::Result;

/// The guide, relative to the repository root.
pub const PAGE: &str = "website/docs/guides/add-an-adapter.md";
/// The report the sample is written from, relative to the repository root.
pub const REPORT: &str = "website/data/conformance/writer-go.json";
/// The line before the generated block.
const BEGIN: &str = "<!-- conform-sample: written by codespec-docs from website/data/conformance/writer-go.json -->";
/// The line after it.
const END: &str = "<!-- /conform-sample -->";

/// The generated block, without the marker lines.
pub fn sample(report: &Report) -> String {
    let summary = serde_json::to_string(&report.summary).expect("a summary always serializes");
    format!(
        "```shell-session\n\
         $ export CODESPEC_ADAPTERS=target/adapters\n\
         $ target/debug/codespec conform --lang go --role writer --out work/go-writer.json\n\
         {}\n\
         work/go-writer.json: written\n\
         $ jq -c .summary work/go-writer.json\n\
         {summary}\n\
         ```\n",
        summary_line(report)
    )
}

/// The guide as it is in `root`, its generated block replaced by [`sample`] of `report`.
pub fn page(root: &Path, report: &Report) -> Result<String> {
    let found = fs::read_to_string(root.join(PAGE)).map_err(|e| format!("{PAGE}: {e}"))?;
    splice(&found, &sample(report))
}

/// `text` with the lines between its one [`BEGIN`] line and the next [`END`] line replaced by
/// `block`.
fn splice(text: &str, block: &str) -> Result<String> {
    let begin = format!("{BEGIN}\n");
    let end = format!("{END}\n");
    let (head, rest) = text
        .split_once(&begin)
        .ok_or_else(|| format!("{PAGE}: no line `{BEGIN}`"))?;
    if rest.contains(&begin) {
        return Err(format!("{PAGE}: more than one line `{BEGIN}`"));
    }
    let (_, tail) = rest
        .split_once(&end)
        .ok_or_else(|| format!("{PAGE}: no line `{END}` after `{BEGIN}`"))?;
    Ok(format!("{head}{begin}{block}{end}{tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespec_conformance::conform::read_report;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn report() -> Report {
        read_report(&fs::read(root().join(REPORT)).unwrap()).unwrap()
    }

    #[test]
    fn the_guide_sample_is_the_checked_in_go_writer_report() {
        let root = root();
        let found = fs::read_to_string(root.join(PAGE)).unwrap();
        let r = report();
        assert_eq!(
            page(&root, &r).unwrap(),
            found,
            "{PAGE}: the conform sample differs from {REPORT}; run `task docs-generate`"
        );
        assert!(found.contains(&summary_line(&r)));
    }

    #[test]
    fn a_new_version_or_count_changes_the_sample() {
        let root = root();
        let found = fs::read_to_string(root.join(PAGE)).unwrap();
        let mut r = report();
        r.adapter.version = "9.9.9".into();
        assert_ne!(page(&root, &r).unwrap(), found);
        let mut r = report();
        r.summary.pass += 1;
        assert_ne!(page(&root, &r).unwrap(), found);
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
