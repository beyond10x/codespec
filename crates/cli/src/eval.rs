//! `codespec eval`: run the calls of a `codespec-calls/1` file against one `cir/1` module with the
//! evaluator (`b10x-codespec-eval`) and print what the conformance kit's runners print.
//!
//! | exit | when |
//! |---|---|
//! | 0 | every call made and printed |
//! | 1 | the run stopped: standard output holds the lines of the calls before the stop, standard error `codespec eval: stopped at <node> (<pos>): <kind>: <reason>` |
//! | 2 | refused before anything ran: the module or the calls file not read, or a call that does not fit the module |

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use codespec_eval::{Options, Run, run_calls};
use codespec_model::Module;

/// The run of `calls` against `module`, or why it is refused.
pub fn eval(module: &Path, calls: &Path, max_steps: u64) -> Result<Run, String> {
    let bytes = std::fs::read(module).map_err(|e| format!("{}: {e}", module.display()))?;
    let m = Module::from_json(&bytes).map_err(|e| format!("{}: {e}", module.display()))?;
    let c = codespec_conformance::read_calls(calls)?;
    run_calls(&m, &c, &Options { max_steps }).map_err(|e| format!("{}: {e}", calls.display()))
}

/// `codespec eval`: prints the run and gives its exit status.
pub fn main(module: &Path, calls: &Path, max_steps: u64) -> ExitCode {
    match eval(module, calls, max_steps) {
        Err(e) => {
            eprintln!("codespec eval: {e}");
            ExitCode::from(2)
        }
        Ok(run) => {
            let mut out = std::io::stdout().lock();
            if out
                .write_all(run.stdout.as_bytes())
                .and_then(|()| out.flush())
                .is_err()
            {
                return ExitCode::FAILURE;
            }
            match run.stop {
                None => ExitCode::SUCCESS,
                Some(stop) => {
                    eprintln!("codespec eval: {stop}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/cases")
            .join(name);
        (dir.join("case.cir.json"), dir.join("calls.json"))
    }

    #[test]
    fn a_kit_case_prints_its_expected_txt() {
        let (module, calls) = case("op-add");
        let run = eval(&module, &calls, 1000).unwrap();
        let expected = std::fs::read_to_string(module.with_file_name("expected.txt")).unwrap();
        assert_eq!(run.stop, None);
        assert_eq!(run.stdout, expected);
    }

    #[test]
    fn a_tight_step_budget_stops_the_run_naming_a_node() {
        let (module, calls) = case("stmt-for");
        let run = eval(&module, &calls, 10).unwrap();
        let stop = run.stop.expect("ten steps do not finish the loop");
        assert_eq!(stop.kind, codespec_eval::StopKind::Budget);
        assert!(stop.node.starts_with("stmt_for.Below#"), "{}", stop.node);
    }

    #[test]
    fn calls_that_do_not_fit_the_module_are_refused() {
        let (module, _) = case("op-add");
        let (_, other) = case("stmt-for");
        let e = eval(&module, &other, 1000).unwrap_err();
        assert!(e.contains("Below"), "{e}");
    }
}
