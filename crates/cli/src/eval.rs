//! `codespec eval`: run the calls of a `codespec-calls/1` file against one `cir/1` module with the
//! evaluator (`b10x-codespec-eval`) and print what the conformance kit's runners print, or run a
//! `codespec-session/1` file and print its trace, `codespec-eval-trace/1`, under a fault plan
//! (`--fault`, `--faults`) if one is given, or once per site and call count (`--each`).
//!
//! | exit | when |
//! |---|---|
//! | 0 | every call made and printed; with `--each`, every run's trace written |
//! | 1 | the run stopped: standard output holds the lines of the calls before the stop (a session's trace ends in its `stop` event), standard error `codespec eval: stopped at <node> (<pos>): <kind>: <reason>`; with `--each`, a run stopped, its line on standard error after `codespec eval: <site> nth <n>:` |
//! | 2 | refused before anything ran: the module, the calls, session, fault plan or sites file not read, a call that does not fit the module, an `--env` that is not `KEY=VALUE`, a `--stub` naming no extern call or of values its results cannot hold, a fault naming no effect site or of a kind its site cannot take, a trace file of `--each` not written |
//!
//! Before a stop's line, standard error has one line `codespec eval: stubbed <key> at <node>
//! (<pos>): <n> calls` for each site a `--stub` answered at. `--each` prints the path of each
//! trace it wrote, one per line.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use codespec_eval::{
    Fault, Options, Run, Session, Stub, read_faults, read_sites, run_calls, run_each, run_session,
};
use codespec_model::Module;

/// What a run makes: the calls of a calls file, a session, or a session once per fault site.
#[derive(Debug, Clone, Copy)]
pub enum Input<'a> {
    Calls(&'a Path),
    Session(&'a Path),
    /// `--each`: the session, the sites file, the last call count and the output directory.
    Each {
        session: &'a Path,
        sites: &'a Path,
        max_nth: u64,
        out: &'a Path,
    },
}

/// The fault plan: `--fault` entries and a `--faults` file.
#[derive(Debug, Clone, Copy, Default)]
pub struct Plan<'a> {
    pub fault: &'a [String],
    pub faults: Option<&'a Path>,
}

/// The options of `--env`, `--stub`, the fault plan and `--max-steps`, or why one is refused.
pub fn options(
    env: &[String],
    stub: &[String],
    plan: &Plan<'_>,
    max_steps: u64,
) -> Result<Options, String> {
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
    let mut faults = match plan.faults {
        Some(p) => {
            let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            read_faults(&bytes).map_err(|e| format!("{}: {e}", p.display()))?
        }
        None => Vec::new(),
    };
    for f in plan.fault {
        faults.push(Fault::parse(f)?);
    }
    Ok(Options {
        max_steps,
        env,
        stubs,
        faults,
    })
}

fn module(path: &Path) -> Result<Module, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Module::from_json(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn session(path: &Path) -> Result<Session, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Session::from_json(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The run of `input` against `module`, or why it is refused; `--each` is [`each`]'s.
pub fn eval(module_path: &Path, input: Input<'_>, opts: &Options) -> Result<Run, String> {
    let m = module(module_path)?;
    match input {
        Input::Calls(calls) => {
            let c = codespec_conformance::read_calls(calls)?;
            run_calls(&m, &c, opts).map_err(|e| format!("{}: {e}", calls.display()))
        }
        Input::Session(path) | Input::Each { session: path, .. } => {
            let s = session(path)?;
            run_session(&m, &s, opts).map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}

/// `--each`: runs the session per site and call count and writes each trace into `out`; gives
/// the paths written and the stops, each after its site and call count.
pub fn each(
    module_path: &Path,
    session_path: &Path,
    sites: &Path,
    max_nth: u64,
    out: &Path,
    opts: &Options,
) -> Result<(Vec<String>, Vec<String>), String> {
    let m = module(module_path)?;
    let s = session(session_path)?;
    let text = std::fs::read_to_string(sites).map_err(|e| format!("{}: {e}", sites.display()))?;
    let sites = read_sites(&text).map_err(|e| format!("{}: {e}", sites.display()))?;
    let runs = run_each(&m, &s, &sites, max_nth, opts)
        .map_err(|e| format!("{}: {e}", session_path.display()))?;
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut written = Vec::with_capacity(runs.len());
    let mut stops = Vec::new();
    for r in &runs {
        let path = out.join(&r.file);
        std::fs::write(&path, &r.run.stdout).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path.display().to_string());
        if let Some(stop) = &r.run.stop {
            stops.push(format!("{} nth {}: {stop}", r.site, r.nth));
        }
    }
    Ok((written, stops))
}

/// `codespec eval`: prints the run and gives its exit status.
pub fn main(
    module: &Path,
    input: Input<'_>,
    env: &[String],
    stub: &[String],
    plan: &Plan<'_>,
    max_steps: u64,
) -> ExitCode {
    let opts = match options(env, stub, plan, max_steps) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("codespec eval: {e}");
            return ExitCode::from(2);
        }
    };
    if let Input::Each {
        session,
        sites,
        max_nth,
        out,
    } = input
    {
        return match each(module, session, sites, max_nth, out, &opts) {
            Err(e) => {
                eprintln!("codespec eval: {e}");
                ExitCode::from(2)
            }
            Ok((written, stops)) => {
                for w in &written {
                    println!("{w}");
                }
                for s in &stops {
                    eprintln!("codespec eval: {s}");
                }
                if stops.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        };
    }
    match eval(module, input, &opts) {
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
            &options(&[], &[], &Plan::default(), max_steps).unwrap(),
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
