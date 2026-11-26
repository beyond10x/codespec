//! The `codespec-corpus` binary: arguments in, `codespec_corpus::run`, a refusal as one line on
//! stderr and exit 1.

use clap::Parser;

fn main() {
    let cli = codespec_corpus::Cli::parse();
    if let Err(e) = codespec_corpus::run(cli) {
        eprintln!("codespec-corpus: {e}");
        std::process::exit(1);
    }
}
