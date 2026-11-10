//! `codespec-write-cir`: the evaluator's identity writer (`codespec_eval::runner`).

use std::io::Read as _;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--describe"] {
        print!("{}", codespec_eval::runner::manifest().to_json());
        return ExitCode::SUCCESS;
    }
    let [out] = args.as_slice() else {
        eprintln!("usage: codespec-write-cir <out dir> | --describe");
        return ExitCode::from(2);
    };
    let mut bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
        eprintln!("codespec-write-cir: stdin: {e}");
        return ExitCode::FAILURE;
    }
    match codespec_eval::runner::write(&bytes, Path::new(out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(lines) => {
            for l in lines {
                eprintln!("codespec-write-cir: {l}");
            }
            ExitCode::FAILURE
        }
    }
}
