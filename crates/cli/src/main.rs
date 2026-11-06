//! `codespec`: validate `cir/1` documents, print the schema, apply passes, run reader and writer
//! adapters, and compare implementations of an example on seeded traces (`harness`).
//!
//! An adapter is a program in whatever language suits the source or target language best. A reader
//! prints one `cir/1` module on stdout; a writer reads one on stdin. This binary validates every
//! document that crosses that boundary, so no adapter's output reaches another unchecked.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use clap::Parser as _;
use codespec_cli::{Cli, Cmd};
use codespec_model::{Module, schema_json};

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
        Cmd::Pass { file, out } => {
            let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let m = read_module(&bytes, &file.display().to_string())?;
            let json = codespec_passes::run(&m).to_json();
            read_module(json.as_bytes(), "pass output")?;
            std::fs::write(&out, json).map_err(|e| format!("{}: {e}", out.display()))?;
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
        Cmd::Harness {
            example,
            seeds,
            commands,
            build_dir,
        } => {
            let summary = codespec_harness::check(&codespec_harness::Options {
                example,
                build_dir,
                seeds,
                commands,
            })
            .map_err(|e| e.to_string())?;
            println!("{summary}");
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
