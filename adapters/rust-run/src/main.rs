//! `codespec-run-rust`: build a written Rust crate and make a conformance case's calls against it
//! (crate doc of `codespec_run_rust`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use codespec_conformance::read_calls;
use codespec_run_rust::Failure;

/// Build a written Rust crate with cargo, make the calls of a codespec-calls/1 document against it
/// and print one line per call in the harness encoding.
#[derive(Parser)]
#[command(name = "codespec-run-rust", version, about)]
struct Cli {
    /// The build directory, emptied first; without it, a temporary directory removed afterwards.
    #[arg(long)]
    build: Option<PathBuf>,
    /// The written crate: Cargo.toml and src/lib.rs.
    dir: PathBuf,
    /// The codespec-calls/1 document.
    calls: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = read_calls(&cli.calls)
        .map_err(Failure::Refused)
        .and_then(|calls| match &cli.build {
            Some(build) => codespec_run_rust::run(&cli.dir, &calls, build),
            None => {
                let tmp =
                    std::env::temp_dir().join(format!("codespec-run-rust-{}", std::process::id()));
                let r = codespec_run_rust::run(&cli.dir, &calls, &tmp);
                let _ = std::fs::remove_dir_all(&tmp);
                r
            }
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec-run-rust: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}
