//! `codespec`: validate `cir/1` documents, print the schema, and run reader and writer adapters.
//!
//! An adapter is a program in whatever language suits the source or target language best. A reader
//! prints one `cir/1` module on stdout; a writer reads one on stdin. This binary validates every
//! document that crosses that boundary, so no adapter's output reaches another unchecked.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use clap::{Parser, Subcommand};
use codespec_model::{Module, schema_json};

#[derive(Parser)]
#[command(
    name = "codespec",
    version,
    about = "Read code into cir/1, check it, write code from it"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate a cir/1 document; with --canonical, also require its canonical encoding.
    Validate {
        file: PathBuf,
        #[arg(long)]
        canonical: bool,
    },
    /// Print the JSON Schema of cir/1.
    Schema,
    /// Run the reader adapter for LANG over SOURCE and write the validated cir/1 module to OUT.
    Read {
        #[arg(long)]
        lang: String,
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Directory holding the adapter programs `codespec-read-<lang>` and `codespec-write-<lang>`.
        #[arg(long, env = "CODESPEC_ADAPTERS")]
        adapters: PathBuf,
    },
    /// Validate a cir/1 module and run the writer adapter for LANG, writing into OUT.
    Write {
        #[arg(long)]
        lang: String,
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, env = "CODESPEC_ADAPTERS")]
        adapters: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec: {e}");
            ExitCode::FAILURE
        }
    }
}

fn read_module(bytes: &[u8], what: &str) -> Result<Module, String> {
    Module::from_json(bytes).map_err(|e| format!("{what}: {e}"))
}

fn run(cmd: Cmd) -> Result<(), String> {
    match cmd {
        Cmd::Validate { file, canonical } => {
            let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let m = read_module(&bytes, &file.display().to_string())?;
            if canonical && m.to_json().as_bytes() != bytes.as_slice() {
                return Err(format!(
                    "{}: valid, but not in canonical encoding",
                    file.display()
                ));
            }
            println!(
                "{}: {} {}, {} types, {} funcs, {} refusals",
                file.display(),
                m.format,
                m.name,
                m.types.len(),
                m.funcs.len(),
                m.refusals.len()
            );
            Ok(())
        }
        Cmd::Schema => {
            print!("{}", schema_json());
            Ok(())
        }
        Cmd::Read {
            lang,
            source,
            out,
            adapters,
        } => {
            let prog = adapter(&adapters, "read", &lang)?;
            let output = Command::new(&prog)
                .arg(&source)
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| format!("{}: {e}", prog.display()))?;
            if !output.status.success() {
                return Err(format!("{} exited with {}", prog.display(), output.status));
            }
            let m = read_module(&output.stdout, &prog.display().to_string())?;
            std::fs::write(&out, m.to_json()).map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(())
        }
        Cmd::Write {
            lang,
            file,
            out,
            adapters,
        } => {
            let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let m = read_module(&bytes, &file.display().to_string())?;
            let prog = adapter(&adapters, "write", &lang)?;
            let mut child = Command::new(&prog)
                .arg(&out)
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|e| format!("{}: {e}", prog.display()))?;
            child
                .stdin
                .take()
                .expect("stdin is piped")
                .write_all(m.to_json().as_bytes())
                .map_err(|e| format!("{}: {e}", prog.display()))?;
            let status = child
                .wait()
                .map_err(|e| format!("{}: {e}", prog.display()))?;
            if !status.success() {
                return Err(format!("{} exited with {status}", prog.display()));
            }
            Ok(())
        }
    }
}

fn adapter(dir: &Path, role: &str, lang: &str) -> Result<PathBuf, String> {
    let p = dir.join(format!("codespec-{role}-{lang}"));
    if p.is_file() {
        Ok(p)
    } else {
        Err(format!(
            "no {role} adapter for {lang:?}: {} does not exist",
            p.display()
        ))
    }
}
