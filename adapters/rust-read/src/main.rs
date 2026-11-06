//! `codespec-read-rust`: reads `<crate dir>/src/lib.rs` and prints one `cir/1` module on stdout.
//!
//! ```console
//! codespec-read-rust <crate dir>
//! ```
//!
//! The module name is the crate's `[package] name` (`-` as `_`). What the reader maps, and how, is
//! the crate doc of the library (`src/lib.rs`). A construct it cannot type or map is kept as a
//! `foreign` statement in Rust with a refusal at its position; a crate that does not parse, or has
//! no `src/lib.rs`, is an error: exit 1, nothing printed.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir] = args.as_slice() else {
        eprintln!("usage: codespec-read-rust <crate dir>");
        return ExitCode::from(2);
    };
    match codespec_read_rust::read_crate(Path::new(dir)) {
        Ok(m) => {
            print!("{}", m.to_json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("codespec-read-rust: {e}");
            ExitCode::FAILURE
        }
    }
}
