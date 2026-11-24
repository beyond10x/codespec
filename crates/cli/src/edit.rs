//! `codespec edit`: the flags, and the outcome of `codespec_edit::run::edit` printed. The pipeline
//! is the library's, so the MCP server runs the same one.

use std::path::PathBuf;
use std::process::ExitCode;

use codespec_edit::run::{Outcome, Request, cache_dir, edit};

/// The arguments of `codespec edit`.
#[derive(clap::Args)]
pub struct Args {
    /// The language of the inputs; selects the reader `codespec-read-<lang>`. Wave 1 runs rust.
    #[arg(long)]
    pub lang: String,
    /// Directory holding the adapter programs `codespec-read-<lang>` and `codespec-write-<lang>`.
    #[arg(long, env = "CODESPEC_ADAPTERS")]
    pub adapters: PathBuf,
    /// No code outside the inputs refers to their exported names; without it, a row whose
    /// target is exported is refused.
    #[arg(long)]
    pub closed_world: bool,
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
    /// The input directories, each read as one module: for Rust, a crate directory.
    #[arg(required = true)]
    pub dirs: Vec<PathBuf>,
}

/// Run `codespec edit`: print the diff (exit 0), or every refusal or the failed verification on
/// standard error (exit 1), or the error (exit 2).
pub fn run(a: &Args) -> ExitCode {
    match outcome(a) {
        Ok(o) => {
            match &o {
                Outcome::NoRow => {}
                Outcome::Verified { diff, .. } => print!("{diff}"),
                Outcome::Refused(reasons) => {
                    for r in reasons {
                        eprintln!("codespec edit: refused: {r}");
                    }
                }
                Outcome::Unverified { reason, .. } => {
                    eprintln!("codespec edit: verification failed: {reason}")
                }
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
        cache,
    })
}
