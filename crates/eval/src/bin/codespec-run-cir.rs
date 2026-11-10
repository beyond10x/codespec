//! `codespec-run-cir`: the evaluator's runner (`codespec_eval::runner`).

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use codespec_eval::runner::{Failed, run};

const USAGE: &str = "usage: codespec-run-cir [--build <dir>] <written dir> <calls.json>";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("--build" | "-build")) {
        if args.len() < 2 {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
        args.drain(..2);
    }
    let [dir, calls] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(Path::new(dir), Path::new(calls)) {
        Err(Failed::Module(e)) => {
            eprintln!("codespec-run-cir: {e}");
            ExitCode::FAILURE
        }
        Err(Failed::Refused(e)) => {
            eprintln!("codespec-run-cir: refused: {e}");
            ExitCode::from(2)
        }
        Ok(r) => {
            let mut out = std::io::stdout().lock();
            if out
                .write_all(r.stdout.as_bytes())
                .and_then(|()| out.flush())
                .is_err()
            {
                return ExitCode::FAILURE;
            }
            match r.stop {
                None => ExitCode::SUCCESS,
                Some(stop) => {
                    eprintln!("codespec-run-cir: {stop}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
