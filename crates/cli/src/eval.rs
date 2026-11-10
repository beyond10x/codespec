//! `codespec eval`: run the calls of a `codespec-calls/1` file against one `cir/1` module with the
//! evaluator (`b10x-codespec-eval`) and print what the conformance kit's runners print, or run a
//! `codespec-session/1` file and print its trace, `codespec-eval-trace/1`.
//!
//! | exit | when |
//! |---|---|
//! | 0 | every call made and printed |
//! | 1 | the run stopped: standard output holds the lines of the calls before the stop (a session's trace ends in its `stop` event), standard error `codespec eval: stopped at <node> (<pos>): <kind>: <reason>` |
//! | 2 | refused before anything ran: the module, the calls or session file not read, a call that does not fit the module, an `--env` that is not `KEY=VALUE`, a `--stub` naming no extern call or of values its results cannot hold |
//!
//! Before a stop's line, standard error has one line `codespec eval: stubbed <key> at <node>
//! (<pos>): <n> calls` for each site a `--stub` answered at.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use codespec_eval::{Options, Run, Session, Stub, run_calls, run_session};
use codespec_model::Module;

/// What a run makes: the calls of a calls file, or a session.
#[derive(Debug, Clone, Copy)]
pub enum Input<'a> {
    Calls(&'a Path),
    Session(&'a Path),
}

/// The options of `--env`, `--stub` and `--max-steps`, or why one is refused.
pub fn options(env: &[String], stub: &[String], max_steps: u64) -> Result<Options, String> {
    let env = env
        .iter()
        .map(|e| match e.split_once('=') {
            Some((k, v)) if !k.is_empty() => Ok((k.to_string(), v.to_string())),
            _ => Err(format!("--env {e:?}: not KEY=VALUE")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stubs = stub
        .iter()
        .map(|s| Stub::parse(s))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Options {
        max_steps,
        env,
        stubs,
    })
}

/// The run of `input` against `module`, or why it is refused.
pub fn eval(module: &Path, input: Input<'_>, opts: &Options) -> Result<Run, String> {
    let bytes = std::fs::read(module).map_err(|e| format!("{}: {e}", module.display()))?;
    let m = Module::from_json(&bytes).map_err(|e| format!("{}: {e}", module.display()))?;
    match input {
        Input::Calls(calls) => {
            let c = codespec_conformance::read_calls(calls)?;
            run_calls(&m, &c, opts).map_err(|e| format!("{}: {e}", calls.display()))
        }
        Input::Session(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let s = Session::from_json(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            run_session(&m, &s, opts).map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}

/// `codespec eval`: prints the run and gives its exit status.
pub fn main(
    module: &Path,
    input: Input<'_>,
    env: &[String],
    stub: &[String],
    max_steps: u64,
) -> ExitCode {
    match options(env, stub, max_steps).and_then(|opts| eval(module, input, &opts)) {
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
            for s in &run.stubbed {
                eprintln!("codespec eval: {s}");
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

    fn run(module: &Path, calls: &Path, max_steps: u64) -> Result<Run, String> {
        eval(
            module,
            Input::Calls(calls),
            &options(&[], &[], max_steps).unwrap(),
        )
    }

    #[test]
    fn a_kit_case_prints_its_expected_txt() {
        let (module, calls) = case("op-add");
        let run = run(&module, &calls, 1000).unwrap();
        let expected = std::fs::read_to_string(module.with_file_name("expected.txt")).unwrap();
        assert_eq!(run.stop, None);
        assert_eq!(run.stdout, expected);
    }

    #[test]
    fn a_tight_step_budget_stops_the_run_naming_a_node() {
        let (module, calls) = case("stmt-for");
        let run = run(&module, &calls, 10).unwrap();
        let stop = run.stop.expect("ten steps do not finish the loop");
        assert_eq!(stop.kind, codespec_eval::StopKind::Budget);
        assert!(stop.node.starts_with("stmt_for.Below#"), "{}", stop.node);
    }

    #[test]
    fn calls_that_do_not_fit_the_module_are_refused() {
        let (module, _) = case("op-add");
        let (_, other) = case("stmt-for");
        let e = run(&module, &other, 1000).unwrap_err();
        assert!(e.contains("Below"), "{e}");
    }
}
