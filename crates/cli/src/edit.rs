//! `codespec edit`: the flags, and the outcome of `codespec_edit::run::edit` printed. The pipeline
//! is the library's, so the MCP server runs the same one.

use std::path::PathBuf;
use std::process::ExitCode;

use codespec_edit::run::{Outcome, Request, Status, cache_dir, edit};

/// The arguments of `codespec edit`.
#[derive(clap::Args)]
pub struct Args {
    /// The language of the inputs; selects the reader `codespec-read-<lang>`. Wave 1 runs rust
    /// and go; a reader of another language that gives no spans is refused (exit 1).
    #[arg(long)]
    pub lang: String,
    /// Directory holding the adapter programs `codespec-read-<lang>` and `codespec-write-<lang>`.
    #[arg(long, env = "CODESPEC_ADAPTERS")]
    pub adapters: PathBuf,
    /// No code outside the inputs refers to their exported names; without it, a row whose
    /// target is exported is refused.
    #[arg(long)]
    pub closed_world: bool,
    /// Write the edited files after verification, each through a temporary file in its
    /// directory and a rename, only when every file read still has the SHA-256 it was read with.
    #[arg(long)]
    pub apply: bool,
    /// Compile the verified copy before anything is written: Go with go build ./... and go vet
    /// ./... (test files included) in the copy of the go.mod root, Rust with cargo check
    /// --all-targets in the copy of the workspace root; each command bounded at 10 minutes. A
    /// failure exits 1 with the first 20 lines of the compiler's output, nothing written.
    #[arg(long)]
    pub build: bool,
    /// text, the unified diff; or json, codespec-edit-report/1 (each row's status and reasons,
    /// each edited file's SHA-256 before and after, the diff).
    #[arg(long, value_parser = ["text", "json"], default_value = "text")]
    pub format: String,
    /// The edit program: cq rules and `edit` items.
    #[arg(
        short = 'e',
        long = "program",
        value_name = "PROGRAM",
        required_unless_present = "file",
        conflicts_with = "file"
    )]
    pub program: Option<String>,
    /// A file holding the edit program.
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<PathBuf>,
    /// The input directories, each read as one module: for Rust a crate directory, for Go a
    /// package directory.
    #[arg(required = true)]
    pub dirs: Vec<PathBuf>,
}

/// Run `codespec edit`: print the diff, or with `--format json` the report (exit 0), or every
/// refusal, the failed verification or the stale file on standard error (exit 1; the report
/// still on standard output with `--format json`), or the error (exit 2).
pub fn run(a: &Args) -> ExitCode {
    match outcome(a) {
        Ok(o) => {
            if a.format == "json" {
                print!("{}", o.report());
            } else if matches!(o.status, Status::Planned | Status::Applied) {
                print!("{}", o.diff);
            }
            for r in &o.reasons {
                eprintln!("codespec edit: {r}");
            }
            ExitCode::from(o.exit_code())
        }
        Err(e) => {
            eprintln!("codespec: {e}");
            ExitCode::from(2)
        }
    }
}

fn outcome(a: &Args) -> Result<Outcome, String> {
    let program = match (&a.program, &a.file) {
        (Some(p), _) => p.clone(),
        (None, Some(path)) => {
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?
        }
        (None, None) => unreachable!("clap requires -e or -f"),
    };
    let cache = cache_dir().ok_or("no cache directory: set CODESPEC_CACHE_DIR")?;
    let cwd = std::env::current_dir().map_err(|e| format!("the current directory: {e}"))?;
    edit(&Request {
        lang: a.lang.clone(),
        adapters: a.adapters.clone(),
        cwd,
        dirs: a.dirs.clone(),
        program,
        closed_world: a.closed_world,
        apply: a.apply,
        build: a.build,
        cache,
    })
}
