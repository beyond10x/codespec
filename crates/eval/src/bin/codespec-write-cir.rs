//! `codespec-write-cir`: the evaluator's identity writer (`codespec_eval::runner`).

use std::io::Read as _;
use std::path::Path;
use std::process::ExitCode;

use codespec_eval::runner::Stops;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--describe"] {
        print!("{}", codespec_eval::runner::manifest().to_json());
        return ExitCode::SUCCESS;
    }
    let (out, stops) = match args.as_slice() {
        [out] => (out, Stops::Refuse),
        [out, flag] if flag == "--stop-when-reached" => (out, Stops::WhenReached),
        _ => {
            eprintln!("usage: codespec-write-cir <out dir> [--stop-when-reached] | --describe");
            return ExitCode::from(2);
        }
    };
    let mut bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
        eprintln!("codespec-write-cir: stdin: {e}");
        return ExitCode::FAILURE;
    }
    match codespec_eval::runner::write(&bytes, Path::new(out), stops) {
        Ok(()) => ExitCode::SUCCESS,
        Err(lines) => {
            for l in lines {
                eprintln!("codespec-write-cir: {l}");
            }
            ExitCode::FAILURE
        }
    }
}
