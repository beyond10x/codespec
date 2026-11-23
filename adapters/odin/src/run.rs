//! Runs codespec-calls/1 on a native Odin package. The sibling reader supplies signatures;
//! only the Odin compiler and the resulting executable determine what a call does.

use std::env;
use std::ffi::OsStr;
use std::fmt::{Display, Write};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

use clap::Parser;
use codespec_conformance::{ArgKind, Calls, check_argument, read_calls};
use serde::Deserialize;

/// Compile the Odin subject with a driver and print the results of its calls.
#[derive(Parser)]
#[command(name = "codespec-run-odin", version, about)]
struct Cli {
    /// Place the isolated native build here. Without it, remove the temporary build on exit.
    #[arg(long)]
    build: Option<PathBuf>,
    /// A package directory or one .odin file.
    source: PathBuf,
    /// A canonical codespec-calls/1 document.
    calls: PathBuf,
}

#[derive(Debug)]
struct Error {
    message: String,
    code: u8,
}

fn refused(message: impl Display) -> Error {
    Error {
        message: message.to_string(),
        code: 2,
    }
}

fn failed(message: impl Display) -> Error {
    Error {
        message: message.to_string(),
        code: 1,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signatures {
    package: String,
    functions: Vec<Signature>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    name: String,
    source: String,
    params: Vec<String>,
    results: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("codespec-run-odin: {}", error.message);
            ExitCode::from(error.code)
        }
    }
}

/// Accumulate only this invocation's files. An existing --build directory may belong
/// to the caller and may hold previous builds.
struct Build {
    root: PathBuf,
    temporary: bool,
}

impl Build {
    fn create(base: Option<&Path>) -> Result<Self, Error> {
        let parent = base.map_or_else(env::temp_dir, Path::to_owned);
        fs::create_dir_all(&parent).map_err(failed)?;
        let parent = fs::canonicalize(parent).map_err(failed)?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        for attempt in 0..1024 {
            let root = parent.join(format!("codespec-odin-{}-{attempt}", std::process::id()));
            match builder.create(&root) {
                Ok(()) => {
                    return Ok(Self {
                        root,
                        temporary: base.is_none(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(failed(error)),
            }
        }
        Err(failed("cannot create a fresh Odin build directory"))
    }
}

impl Drop for Build {
    fn drop(&mut self) {
        if self.temporary {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn copy_source(source: &Path, subject: &Path) -> Result<(), Error> {
    let is_odin = |path: &Path| path.extension() == Some(OsStr::new("odin"));
    let meta = fs::symlink_metadata(source).map_err(refused)?;
    let files = if meta.is_file() && is_odin(source) {
        vec![source.to_owned()]
    } else if meta.is_dir() {
        let mut files = Vec::new();
        for entry in fs::read_dir(source).map_err(refused)? {
            let entry = entry.map_err(refused)?;
            if is_odin(&entry.path()) {
                if !entry.file_type().map_err(refused)?.is_file() {
                    return Err(refused("expected a regular .odin source file"));
                }
                files.push(entry.path());
            }
        }
        files
    } else {
        return Err(refused(
            "expected an Odin package directory or a regular .odin file",
        ));
    };
    if files.is_empty() {
        return Err(refused("no .odin files in source directory"));
    }
    fs::create_dir(subject).map_err(failed)?;
    for file in &files {
        fs::copy(
            file,
            subject.join(file.file_name().expect("source file has a name")),
        )
        .map_err(failed)?;
    }
    Ok(())
}

fn diagnostic(program: &str, output: &Output) -> String {
    format!(
        "{program} exited {}: {}{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    )
    .trim_end()
    .to_owned()
}

fn signatures(subject: &Path) -> Result<Signatures, Error> {
    let name = format!("codespec-read-odin{}", env::consts::EXE_SUFFIX);
    let sibling = env::current_exe().map_err(failed)?.with_file_name(&name);
    let reader = if sibling.is_file() {
        sibling
    } else {
        env::var_os("CODESPEC_ADAPTERS")
            .map(|dir| PathBuf::from(dir).join(&name))
            .ok_or_else(|| failed(format!("missing sibling adapter {name}")))?
    };
    let output = Command::new(reader)
        .arg("--signatures")
        .arg(subject)
        .output()
        .map_err(failed)?;
    if !output.status.success() {
        let message = diagnostic("codespec-read-odin --signatures", &output);
        return Err(if output.status.code() == Some(2) {
            refused(message)
        } else {
            failed(message)
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| failed(format!("invalid Odin signatures: {error}")))
}

fn run(cli: &Cli) -> Result<(), Error> {
    let calls = read_calls(&cli.calls).map_err(refused)?;
    let build = Build::create(cli.build.as_deref())?;
    let subject = build.root.join("subject");
    copy_source(&cli.source, &subject)?;
    let driver = driver(&signatures(&subject)?, &calls)?;
    fs::write(build.root.join("main.odin"), driver).map_err(failed)?;

    let binary = format!("runner{}", env::consts::EXE_SUFFIX);
    let output = Command::new("odin")
        .current_dir(&build.root)
        .args(["build", ".", &format!("-out:{binary}")])
        .output()
        .map_err(|error| failed(format!("odin build: {error}")))?;
    if !output.status.success() {
        return Err(failed(diagnostic("odin build", &output)));
    }
    let status = Command::new(build.root.join(binary))
        .current_dir(&build.root)
        .status()
        .map_err(failed)?;
    if !status.success() {
        return Err(failed(format!("Odin subject exited {status}")));
    }
    Ok(())
}

fn identifier(name: &str) -> bool {
    let mut letters = name.bytes();
    letters
        .next()
        .is_some_and(|b| b == b'_' || b.is_ascii_alphabetic())
        && letters.all(|b| b == b'_' || b.is_ascii_alphanumeric())
}

fn argument(arg: &str, typ: &str) -> Result<(), Error> {
    let kind = match typ {
        "int" | "i64" | "i32" | "i16" | "i8" => ArgKind::Int,
        "bool" => ArgKind::Bool,
        "string" => ArgKind::String,
        _ => return Err(refused(format!("unsupported Odin parameter type {typ:?}"))),
    };
    check_argument(arg, kind).map_err(refused)?;
    let fits = match typ {
        "int" => arg.parse::<isize>().is_ok(),
        "i32" => arg.parse::<i32>().is_ok(),
        "i16" => arg.parse::<i16>().is_ok(),
        "i8" => arg.parse::<i8>().is_ok(),
        _ => true,
    };
    if !fits {
        return Err(refused(format!("argument {arg} does not fit {typ}")));
    }
    Ok(())
}

/// Arguments become Odin literals only after check_argument accepts their entire text.
/// Only a source identifier supplied by the native parser may name the called procedure.
fn driver(signatures: &Signatures, calls: &Calls) -> Result<String, Error> {
    if !identifier(&signatures.package) {
        return Err(refused("invalid Odin package name"));
    }
    let mut body = String::from("main :: proc() {\n");
    let mut strings = false;
    for call in &calls.calls {
        let mut candidates = signatures
            .functions
            .iter()
            .filter(|sig| sig.name == call.func || sig.source == call.func);
        let sig = candidates
            .next()
            .ok_or_else(|| refused(format!("no exported procedure {:?}", call.func)))?;
        if candidates.next().is_some() || !identifier(&sig.source) {
            return Err(refused(format!(
                "ambiguous or invalid source name for {:?}",
                call.func
            )));
        }
        if sig.params.len() != call.args.len() {
            return Err(refused(format!("wrong argument count for {:?}", call.func)));
        }
        for (arg, typ) in call.args.iter().zip(&sig.params) {
            argument(arg, typ)?;
        }

        body.push_str("    {\n        ");
        let vars: Vec<_> = (0..sig.results.len())
            .map(|index| format!("v{index}"))
            .collect();
        if !vars.is_empty() {
            write!(body, "{} := ", vars.join(", ")).unwrap();
        }
        writeln!(body, "subject.{}({})", sig.source, call.args.join(", ")).unwrap();
        body.push_str("        fmt.print(\"ok\")\n");
        for (var, typ) in vars.iter().zip(&sig.results) {
            body.push_str("        fmt.print(\" \")\n");
            match typ.as_str() {
                "int" | "i8" | "i16" | "i32" | "i64" | "bool" => {
                    writeln!(body, "        fmt.print({var})").unwrap()
                }
                "string" => {
                    strings = true;
                    writeln!(body, "        print_quoted({var})").unwrap()
                }
                _ => return Err(refused(format!("unsupported Odin result type {typ:?}"))),
            }
        }
        body.push_str("        fmt.println()\n    }\n");
    }
    body.push_str("}\n");
    let text = if strings { QUOTE } else { "" };
    Ok(format!(
        "package main\nimport subject \"subject\"\nimport \"core:fmt\"\n{text}\n{body}"
    ))
}

/// Go strconv.Quote. Work on UTF-8 bytes to preserve malformed input, and keep letter,
/// mark, number, punctuation and symbol runes as their original UTF-8 sequence.
const QUOTE: &str = r#"
import "core:unicode"
import "core:unicode/utf8"

print_quoted :: proc(value: string) {
    fmt.print("\"")
    for i := 0; i < len(value); {
        r, n := utf8.decode_rune_in_string(value[i:])
        if r == utf8.RUNE_ERROR && n == 1 {
            fmt.printf("\\x%02x", value[i])
            i += 1
            continue
        }
        switch r {
        case '"':  fmt.print("\\\"")
        case '\\': fmt.print("\\\\")
        case '\a': fmt.print("\\a")
        case '\b': fmt.print("\\b")
        case '\t': fmt.print("\\t")
        case '\n': fmt.print("\\n")
        case '\v': fmt.print("\\v")
        case '\f': fmt.print("\\f")
        case '\r': fmt.print("\\r")
        case:
            if r == ' ' || (unicode.is_graphic(r) && !unicode.is_space(r)) {
                fmt.print(value[i:i+n])
            } else if r < 32 || r == 127 {
                fmt.printf("\\x%02x", u32(r))
            } else if u32(r) < 0x10000 {
                fmt.printf("\\u%04x", u32(r))
            } else {
                fmt.printf("\\U%08x", u32(r))
            }
        }
        i += n
    }
    fmt.print("\"")
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use codespec_conformance::Call;

    #[test]
    fn arguments_are_checked_for_type_width_and_encoding_before_emission() {
        for (arg, typ) in [
            ("128", "i8"),
            ("01", "int"),
            ("true", "string"),
            ("err()", "int"),
        ] {
            assert!(argument(arg, typ).is_err(), "{arg} {typ}");
        }
        assert!(argument("-128", "i8").is_ok());
        assert!(argument(r#""' and spaces""#, "string").is_ok());
    }

    #[test]
    fn calls_use_the_declared_spelling_and_all_result_values() {
        let sigs = Signatures {
            package: "sample".into(),
            functions: vec![Signature {
                name: "ReadPair".into(),
                source: "read_pair".into(),
                params: vec!["bool".into()],
                results: vec!["string".into(), "i32".into()],
            }],
        };
        let calls = Calls {
            format: "codespec-calls/1".into(),
            calls: ["ReadPair", "read_pair"]
                .map(|name| Call {
                    func: name.into(),
                    args: vec!["false".into()],
                })
                .to_vec(),
        };
        let code = driver(&sigs, &calls).unwrap();
        assert_eq!(
            code.matches("v0, v1 := subject.read_pair(false)").count(),
            2
        );
        assert!(code.contains("print_quoted(v0)"));
        assert!(code.contains("fmt.print(v1)"));
    }
}
