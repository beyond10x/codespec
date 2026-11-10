//! The evaluator held to the conformance kit (requirements E1, E2 and E4): every case run twice
//! on its `calls.json` prints its `expected.txt` with the same bytes both times, except a case
//! whose run executes a `foreign` statement, which is listed with the stop it must give.
//!
//! # The list of stops
//!
//! One line per listed case, `<case> <stop>`, the stop as [`Stop`]'s `Display` writes it (the line
//! `codespec eval` prints on standard error after `codespec eval: `):
//!
//! ```text
//! stmt-foreign stopped at stmt_foreign.Count#1 (case.go:6): foreign: defer is outside cir/1 (go: `defer done()`)
//! ```
//!
//! [`check`] fails a case when
//!
//! - it is listed and stops with another line, or does not stop and prints something other than
//!   its `expected.txt`;
//! - it is listed and stops with its line, but what it printed before the stop does not begin
//!   its `expected.txt`;
//! - it is listed and prints its `expected.txt` after all: the entry is stale;
//! - it is not listed and stops;
//! - it is not listed and prints something other than its `expected.txt`;
//! - its two runs differ;
//!
//! and the list fails when it names a case twice, names a case that is not among the cases
//! checked, or holds a line that is not `<case> <stop>`. `task eval-kit` runs this check through
//! the test `tests/kit.rs`.

use std::collections::BTreeMap;

use codespec_conformance::Case;

use crate::{Options, Run, Stop, run_calls};

/// What [`check`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub cases: usize,
    /// Cases that printed their `expected.txt`.
    pub printed: usize,
    /// Listed cases that stopped with their listed line.
    pub stopped: usize,
    /// One line per failure, naming the case or the list.
    pub failed: Vec<String>,
}

impl Report {
    /// `eval-kit: <n> cases, <p> printed, <s> stopped as listed, <f> failed`.
    pub fn summary(&self) -> String {
        format!(
            "eval-kit: {} cases, {} printed, {} stopped as listed, {} failed",
            self.cases,
            self.printed,
            self.stopped,
            self.failed.len()
        )
    }
}

/// The list of stops, `case` → stop line; a line that is not `<case> <stop>` or a case named
/// twice is an error.
pub fn read_stops(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((case, stop)) = line.split_once(' ') else {
            return Err(format!("kit-stops line {}: not `<case> <stop>`", i + 1));
        };
        if !stop.starts_with("stopped at ") {
            return Err(format!(
                "kit-stops line {}: the stop of {case} does not start `stopped at `",
                i + 1
            ));
        }
        if out.insert(case.to_string(), stop.to_string()).is_some() {
            return Err(format!("kit-stops: {case} is listed twice"));
        }
    }
    Ok(out)
}

/// Run every case of `cases` twice and judge it against its `expected.txt` and the list of
/// stops `stops` (module doc).
pub fn check(cases: &[Case], stops: &str) -> Report {
    let mut report = Report {
        cases: cases.len(),
        printed: 0,
        stopped: 0,
        failed: Vec::new(),
    };
    let listed = match read_stops(stops) {
        Ok(l) => l,
        Err(e) => {
            report.failed.push(e);
            return report;
        }
    };
    for name in listed.keys() {
        if !cases.iter().any(|c| c.name == *name) {
            report
                .failed
                .push(format!("kit-stops: {name} is not a case of the kit"));
        }
    }
    for case in cases {
        let expected = match std::fs::read_to_string(case.dir.join("expected.txt")) {
            Ok(e) => e,
            Err(e) => {
                report
                    .failed
                    .push(format!("{}: expected.txt: {e}", case.name));
                continue;
            }
        };
        let runs = (
            run_calls(&case.module, &case.calls, &Options::default()),
            run_calls(&case.module, &case.calls, &Options::default()),
        );
        let (a, b) = match runs {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                report.failed.push(format!("{}: refused: {e}", case.name));
                continue;
            }
        };
        if a != b {
            report
                .failed
                .push(format!("{}: a second run printed other bytes", case.name));
            continue;
        }
        match judge(&a, &expected, listed.get(&case.name).map(String::as_str)) {
            Ok(Judged::Printed) => report.printed += 1,
            Ok(Judged::Stopped) => report.stopped += 1,
            Err(why) => report.failed.push(format!("{}: {why}", case.name)),
        }
    }
    report
}

enum Judged {
    Printed,
    Stopped,
}

fn judge(run: &Run, expected: &str, listed: Option<&str>) -> Result<Judged, String> {
    let stop = run.stop.as_ref().map(Stop::to_string);
    match (listed, stop) {
        (None, None) if run.stdout == expected => Ok(Judged::Printed),
        (None, None) => Err(format!(
            "printed {:?}, expected.txt holds {expected:?}",
            run.stdout
        )),
        (None, Some(s)) => Err(format!("stopped, and is not listed in kit-stops: {s}")),
        (Some(_), None) if run.stdout == expected => Err(
            "listed as stopping, but prints its expected.txt: remove its kit-stops entry".into(),
        ),
        (Some(want), None) => Err(format!(
            "listed as stopping with {want:?}, but ran to the end and printed {:?}",
            run.stdout
        )),
        (Some(want), Some(s)) if s == want && !expected.starts_with(&run.stdout) => Err(format!(
            "stopped as listed, but printed {:?} before the stop, which does not begin \
             expected.txt's {expected:?}",
            run.stdout
        )),
        (Some(want), Some(s)) if s == want => Ok(Judged::Stopped),
        (Some(want), Some(s)) => Err(format!("stopped with {s:?}, not the listed {want:?}")),
    }
}
