//! Requirements E1 and E2 (`docs/evaluator.md` § 2): every conformance kit case evaluated on its
//! `calls.json` prints its `expected.txt`, and a second run prints the same bytes. The one
//! exception is data: a case whose run executes a `foreign` statement is listed in
//! `tests/kit-stops.txt` with the stop it must give (E4).

use std::path::Path;

use codespec_conformance::{Case, load_cases};
use codespec_eval::kit;

const STOPS: &str = include_str!("kit-stops.txt");

fn cases() -> Vec<Case> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    load_cases(&dir).expect("the kit reads")
}

#[test]
fn every_kit_case_prints_its_expected_txt_twice_byte_equal_or_stops_as_listed() {
    let cases = cases();
    assert!(!cases.is_empty(), "no kit case");
    let report = kit::check(&cases, STOPS);
    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "{}\n{}",
        report.summary(),
        report.failed.join("\n")
    );
    assert_eq!(report.cases, cases.len());
    assert_eq!(report.printed + report.stopped, cases.len());
}

/// The check on a fixture list, over the kit cases `op-add` (prints) and `stmt-foreign` (stops):
/// each of its three rules fails the run, and a right list passes.
#[test]
fn the_kit_check_fails_a_wrong_stop_a_listed_case_that_prints_and_an_unlisted_stop() {
    let two: Vec<Case> = cases()
        .into_iter()
        .filter(|c| c.name == "op-add" || c.name == "stmt-foreign")
        .collect();
    assert_eq!(two.len(), 2);
    let right = STOPS
        .lines()
        .find(|l| l.starts_with("stmt-foreign "))
        .expect("stmt-foreign is listed");

    let report = kit::check(&two, right);
    assert_eq!(report.failed, Vec::<String>::new());
    assert_eq!((report.printed, report.stopped), (1, 1));
    assert_eq!(
        report.summary(),
        "eval-kit: 2 cases, 1 printed, 1 stopped as listed, 0 failed"
    );

    let wrong_line = "stmt-foreign stopped at stmt_foreign.Count#2 (case.go:6): foreign: other";
    let report = kit::check(&two, wrong_line);
    assert_eq!(report.failed.len(), 1, "{:?}", report.failed);
    assert!(
        report.failed[0].starts_with("stmt-foreign: stopped with")
            && report.failed[0].contains("not the listed"),
        "{}",
        report.failed[0]
    );

    let prints = format!("{right}\nop-add stopped at op_add.Add#0 (case.go:5): foreign: none");
    let report = kit::check(&two, &prints);
    assert_eq!(report.failed.len(), 1, "{:?}", report.failed);
    assert!(
        report.failed[0].starts_with("op-add: listed as stopping, but prints its expected.txt"),
        "{}",
        report.failed[0]
    );

    let report = kit::check(&two, "");
    assert_eq!(report.failed.len(), 1, "{:?}", report.failed);
    assert!(
        report.failed[0].starts_with("stmt-foreign: stopped, and is not listed"),
        "{}",
        report.failed[0]
    );
    assert_eq!(
        report.summary(),
        "eval-kit: 2 cases, 1 printed, 0 stopped as listed, 1 failed"
    );
}
