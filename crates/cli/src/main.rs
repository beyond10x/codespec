//! `codespec`: validate `cir/1` documents, print the schema and the feature vocabulary, apply
//! passes, run reader and writer adapters, compare implementations of an example on seeded traces
//! (`harness`), run the conformance kit against an adapter (`conform`), measure how much of a
//! module a reader read (`measure`), run the built-in defect checks (`check`), compare the code
//! facts of two revisions (`diff`) and change source code with a cq edit program (`edit`).
//!
//! An adapter is a program in whatever language suits the source or target language best. A reader
//! prints one `cir/1` module on stdout; a writer reads one on stdin. This binary validates every
//! document that crosses that boundary, so no adapter's output reaches another unchecked.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use clap::Parser as _;
use codespec_cli::{Cli, Cmd};
use codespec_conformance::conform::{
    Reason, Target, Verdict, check_work, conform_adapter_within, differences, read_report,
    report_json, summary_line,
};
use codespec_model::{Module, features_json, schema_json};

/// On musl, mimalloc: musl's own allocator made `query`, `effects` and `diff` over a large set
/// slower (the numbers: `docs/release.md`). Other targets keep the system allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    let command = Cli::parse().command;
    if let Cmd::Check { .. } = command {
        return match check(command) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("codespec: {e}");
                ExitCode::from(2)
            }
        };
    }
    if let Cmd::Diff { .. } = command {
        return match diff(command) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("codespec: {e}");
                ExitCode::from(2)
            }
        };
    }
    if let Cmd::Query(args) = &command {
        return match codespec_cli::query::run(args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("codespec: {e}");
                ExitCode::from(2)
            }
        };
    }
    if let Cmd::Edit(args) = &command {
        return codespec_cli::edit::run(args);
    }
    if let Cmd::Eval {
        module,
        calls,
        session,
        env,
        stub,
        fault,
        faults,
        each,
        sites,
        max_nth,
        out,
        max_steps,
    } = &command
    {
        use codespec_cli::eval::Input;
        let input = match (calls, session, sites, out) {
            (Some(c), ..) => Input::Calls(c),
            (None, Some(s), Some(sites), Some(out)) if *each => Input::Each {
                session: s,
                sites,
                max_nth: *max_nth,
                out,
            },
            (None, Some(s), ..) => Input::Session(s),
            (None, None, ..) => unreachable!("clap requires --calls or --session"),
        };
        let plan = codespec_cli::eval::Plan {
            fault,
            faults: faults.as_deref(),
        };
        return codespec_cli::eval::main(module, input, env, stub, &plan, *max_steps);
    }
    match run(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `codespec check`: exit 0 with no finding, 1 with findings; an input error is the caller's 2.
fn check(command: Cmd) -> Result<ExitCode, String> {
    let Cmd::Check {
        check,
        modules,
        list,
        format,
        strict,
        names,
        report,
        entry,
        exported,
    } = command
    else {
        unreachable!("main passes only check");
    };
    let names = match names {
        Some(path) => {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            codespec_check::Names::from_json(&bytes)
                .map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => codespec_check::Names::default(),
    };
    if list {
        print!("{}", codespec_check::list_text(&names));
        return Ok(ExitCode::SUCCESS);
    }
    let check = check.expect("clap requires a check without --list");
    let set = read_set(&modules)?;
    let opts = codespec_check::Options {
        strict,
        names,
        report,
        entries: entry,
        exported,
    };
    let r = codespec_check::run(&check, &set, &opts)?;
    if format == "json" {
        print!("{}", r.to_json());
    } else {
        print!("{}", r.to_text());
    }
    Ok(if r.findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// `codespec diff`: exit 0 with no delta, 1 with deltas (or `unknowns` counts); an input error is
/// the caller's 2.
fn diff(command: Cmd) -> Result<ExitCode, String> {
    let Cmd::Diff {
        old,
        new,
        summaries,
        barrier,
        no_default_summaries,
        entry,
        exported,
        dispatch,
        report,
        format,
        unknowns,
    } = command
    else {
        unreachable!("main passes only diff");
    };
    let old = codespec_diff::read_dir(&old)?;
    let new = codespec_diff::read_dir(&new)?;
    let (summaries, barrier) = with_defaults(summaries, barrier, no_default_summaries)?;
    let barrier = read_barrier(barrier.as_deref())?;
    let summaries = read_summaries(&summaries)?;
    let opts = codespec_diff::Options {
        summaries: &summaries,
        barrier: barrier.as_ref(),
        report,
        entries: entry,
        exported,
        dispatch: dispatch_mode(Some(&dispatch), true),
    };
    let mut r = codespec_diff::diff(&old, &new, &opts)?;
    if unknowns == "summary" {
        r.summarize_unknowns();
    }
    if format == "json" {
        print!("{}", r.to_json());
    } else {
        print!("{}", r.to_text());
    }
    Ok(if r.deltas.is_empty() && r.unknowns.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn read_module(bytes: &[u8], what: &str) -> Result<Module, String> {
    Module::from_json(bytes).map_err(|e| format!("{what}: {e}"))
}

fn read_set(paths: &[PathBuf]) -> Result<Vec<Module>, String> {
    let mut set = Vec::with_capacity(paths.len());
    for p in paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        set.push(read_module(&bytes, &p.display().to_string())?);
    }
    Ok(set)
}

fn read_barrier(path: Option<&Path>) -> Result<Option<codespec_effects::Barrier>, String> {
    let Some(path) = path else { return Ok(None) };
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    codespec_effects::barrier_from_json(&bytes)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The summaries and barrier `diff` and `effects` read: each flag as given; for one not given,
/// unless `off`, the files a release installs in `share/codespec/summaries/` beside the
/// executable (every `*.effects.json` in name order, the one `*.barrier.json`). Without that
/// directory, none: a build from the tree finds no default, never the tree's `summaries/`.
fn with_defaults(
    summaries: Vec<PathBuf>,
    barrier: Option<PathBuf>,
    off: bool,
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    if off || (!summaries.is_empty() && barrier.is_some()) {
        return Ok((summaries, barrier));
    }
    let Some(dir) = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("share/codespec/summaries")))
        .filter(|dir| dir.is_dir())
    else {
        return Ok((summaries, barrier));
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    files.sort();
    let named = |suffix: &str| -> Vec<PathBuf> {
        files
            .iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(suffix))
            })
            .cloned()
            .collect()
    };
    let summaries = if summaries.is_empty() {
        named(".effects.json")
    } else {
        summaries
    };
    let barrier = match barrier {
        Some(b) => Some(b),
        None => {
            let mut found = named(".barrier.json");
            if found.len() > 1 {
                return Err(format!(
                    "{}: {} barrier documents; name one with --barrier",
                    dir.display(),
                    found.len()
                ));
            }
            found.pop()
        }
    };
    Ok((summaries, barrier))
}

fn read_summaries(paths: &[PathBuf]) -> Result<Vec<codespec_effects::Summary>, String> {
    paths
        .iter()
        .map(|path| {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            codespec_effects::summary_from_json(&bytes)
                .map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect()
}

/// `cha` or `rta` as given; unset, rta when there are entries and cha otherwise.
fn dispatch_mode(given: Option<&str>, with_entries: bool) -> codespec_effects::DispatchMode {
    match given {
        Some("rta") => codespec_effects::DispatchMode::Rta,
        Some(_) => codespec_effects::DispatchMode::Cha,
        None if with_entries => codespec_effects::DispatchMode::Rta,
        None => codespec_effects::DispatchMode::Cha,
    }
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
        Cmd::Features => {
            print!("{}", features_json());
            Ok(())
        }
        Cmd::Read {
            lang,
            source,
            out,
            spans,
            adapters,
        } => {
            let prog = adapter(&adapters, "read", &lang)?;
            if let Some(spans) = spans {
                let read = codespec_edit::run::read_spanned(&prog, &source)?;
                for d in &read.dropped {
                    eprintln!("{d}");
                }
                std::fs::write(&out, read.cir_json())
                    .map_err(|e| format!("{}: {e}", out.display()))?;
                std::fs::write(&spans, read.spans_json())
                    .map_err(|e| format!("{}: {e}", spans.display()))?;
                return Ok(());
            }
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
        Cmd::Conform {
            lang,
            role,
            out,
            check,
            kit,
            corpus,
            adapters,
            work,
            timeout,
        } => conform(Conform {
            lang,
            role,
            out,
            check,
            kit,
            corpus,
            adapters,
            work,
            timeout,
        }),
        Cmd::Measure { module, format } => {
            let bytes = std::fs::read(&module).map_err(|e| format!("{}: {e}", module.display()))?;
            let m = read_module(&bytes, &module.display().to_string())?;
            let report = codespec_cli::measure::measure(&m);
            if format == "json" {
                print!("{}", codespec_cli::measure::json(&report));
            } else {
                print!("{}", codespec_cli::measure::text(&report));
            }
            Ok(())
        }
        Cmd::Effects {
            modules,
            summaries,
            barrier,
            no_default_summaries,
            report,
            roots,
            entry,
            exported,
            dispatch,
        } => {
            let set = read_set(&modules)?;
            let (summaries, barrier) = with_defaults(summaries, barrier, no_default_summaries)?;
            let barrier = read_barrier(barrier.as_deref())?;
            let read = read_summaries(&summaries)?;
            let with_entries = !entry.is_empty() || !exported.is_empty();
            let opts = codespec_effects::Options {
                barrier: barrier.as_ref(),
                report,
                roots,
                entries: entry,
                exported,
                dispatch: dispatch_mode(dispatch.as_deref(), with_entries),
            };
            codespec_effects::check_entries(&set, &opts)?;
            print!(
                "{}",
                codespec_effects::analyze_set(&set, &read, &opts).to_json()
            );
            Ok(())
        }
        Cmd::Reach {
            modules,
            entry,
            exported,
            dispatch,
            report,
        } => {
            let set = read_set(&modules)?;
            let opts = codespec_effects::Options {
                report,
                entries: entry,
                exported,
                dispatch: dispatch_mode(Some(&dispatch), true),
                ..Default::default()
            };
            print!("{}", codespec_effects::reach(&set, &opts)?.to_json());
            Ok(())
        }
        Cmd::Trace {
            modules,
            from,
            depth,
            summaries,
            barrier,
            dispatch,
        } => {
            let set = read_set(&modules)?;
            let barrier = read_barrier(barrier.as_deref())?;
            let read = read_summaries(&summaries)?;
            let opts = codespec_effects::Options {
                barrier: barrier.as_ref(),
                dispatch: dispatch_mode(Some(&dispatch), true),
                ..Default::default()
            };
            let t = codespec_effects::trace(&set, &read, &opts, &from, depth)?;
            print!("{}", t.to_json());
            Ok(())
        }
        Cmd::Check { .. } => check(cmd).map(|_| ()),
        Cmd::Diff { .. } => unreachable!("main runs diff"),
        Cmd::Query(args) => codespec_cli::query::run(&args).map(|_| ()),
        Cmd::Eval { .. } => unreachable!("main runs eval"),
        Cmd::Edit(_) => unreachable!("main runs edit"),
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

/// The arguments of `codespec conform`.
struct Conform {
    lang: Option<String>,
    role: Option<String>,
    out: Option<PathBuf>,
    check: Option<PathBuf>,
    kit: PathBuf,
    corpus: Option<PathBuf>,
    adapters: PathBuf,
    work: Option<PathBuf>,
    timeout: u64,
}

fn conform(c: Conform) -> Result<(), String> {
    let (old, baseline) = match &c.check {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let r = read_report(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            (Some(r), bytes)
        }
        None => (None, Vec::new()),
    };
    let (role, lang) = match &old {
        Some(r) => (r.adapter.role.clone(), r.adapter.lang.clone()),
        None => (
            c.role.expect("clap requires --role without --check"),
            c.lang.expect("clap requires --lang without --check"),
        ),
    };
    let work = c
        .work
        .unwrap_or_else(|| PathBuf::from(format!("target/conform/{role}-{lang}")));
    // The run empties the work directory first: a report inside it would be erased by the run
    // that checks or writes it.
    let reports: Vec<(&str, &Path)> = [&c.check, &c.out]
        .into_iter()
        .flatten()
        .map(|p| ("report", p.as_path()))
        .collect();
    check_work(&work, &reports)?;
    let report = conform_adapter_within(
        &Target {
            adapters: c.adapters,
            role,
            lang,
            kit: c.kit,
            corpus: c.corpus,
            work,
        },
        std::time::Duration::from_secs(c.timeout),
    )?;
    println!("{}", summary_line(&report));
    if let (Some(old), Some(path)) = (old, &c.check) {
        let diff = differences(&old, &report);
        if diff.is_empty() {
            if std::fs::read(path).ok().as_ref() != Some(&baseline) {
                return Err(format!(
                    "{}: gone or changed during the run, so it is not current",
                    path.display()
                ));
            }
            println!("{}: current", path.display());
            return Ok(());
        }
        for d in &diff {
            eprintln!("{d}");
        }
        return Err(format!(
            "{}: stale, {} difference(s); write it again with --out",
            path.display(),
            diff.len()
        ));
    }
    let out = c.out.expect("clap requires --out without --check");
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&out, report_json(&report)).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}: written", out.display());
    if report.conforms {
        return Ok(());
    }
    for f in report
        .features
        .iter()
        .filter(|f| f.verdict == Verdict::Fail)
    {
        let mut why = f.reason.unwrap_or(Reason::NoCase).to_string();
        if let Some(case) = &f.case {
            why = format!("{case}: {why}");
        }
        if let Some(d) = &f.first_difference {
            why += &format!(
                ", line {} expected {:?}, got {:?}",
                d.line, d.expected, d.got
            );
        }
        eprintln!("FAIL {} (claimed: {}): {why}", f.id, f.claimed);
    }
    for b in report
        .boundary
        .iter()
        .filter(|b| b.verdict == Verdict::Fail)
    {
        eprintln!(
            "FAIL boundary {}: {}",
            b.id,
            b.detail.as_deref().unwrap_or("")
        );
    }
    Err(format!(
        "{} {} does not conform",
        report.adapter.role, report.adapter.lang
    ))
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
