//! `codespec-run-c`: compile C written by `codespec-write-c`, or by hand, and make a conformance
//! case's calls against it (crate doc of `codespec_run_c`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use codespec_conformance::read_calls;
use codespec_run_c::Failure;

/// Compile C written by codespec-write-c, or by hand, with a driver that makes the calls of a codespec-calls/1
/// document and prints one line per call in the harness encoding.
#[derive(Parser)]
#[command(name = "codespec-run-c", version, about)]
struct Cli {
    /// The build directory, emptied first; without it, a temporary directory removed afterwards.
    #[arg(long)]
    build: Option<PathBuf>,
    /// The written directory (<module>.c, <module>.h and codespec_rt.h), or a directory of C
    /// sources without a module header.
    dir: PathBuf,
    /// The codespec-calls/1 document.
    calls: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = read_calls(&cli.calls)
        .map_err(Failure::Refused)
        .and_then(|calls| match &cli.build {
            Some(build) => codespec_run_c::run(&cli.dir, &calls, build),
            None => {
                let tmp =
                    std::env::temp_dir().join(format!("codespec-run-c-{}", std::process::id()));
                let r = codespec_run_c::run(&cli.dir, &calls, &tmp);
                let _ = std::fs::remove_dir_all(&tmp);
                r
            }
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec-run-c: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
