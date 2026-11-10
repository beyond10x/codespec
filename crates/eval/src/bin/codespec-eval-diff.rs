//! `codespec-eval-diff`: per-function differential runs of examples through the writer and
//! runner pairs of `cir` (the evaluator), `go` and `rust` (`codespec_eval::diff`).
//!
//! ```console
//! codespec-eval-diff --adapters <dir> --build-dir <dir> [--seeds <n>] [--refused <example>:<lang>:<function>]… <example dir>…
//! ```
//!
//! For each example directory `<x>`, its passed module `<x>/<name>.cir.json` (`name` the
//! directory's name) gets its probes and `n` calls per entry (default 200), written under
//! `<build-dir>/<name>/` as `probes.cir.json` and `calls.json`. Each language's writer writes the
//! probe module into `<lang>/out`; its runner runs the calls with `<lang>/build` as its build
//! directory and must exit 0. The evaluator's output must hold one line per call, and each other
//! language's output must equal the evaluator's lines for the calls it ran, byte for byte.
//!
//! A writer refuses nothing unless `--refused` lists it: `<example>:<lang>:<function key>`
//! (`inventory:rust:Inventory.Find`) names an entry that language's writer refuses, whose calls
//! that language then does not run. A listed entry the writer does not refuse fails the run, as
//! does a refusal not listed. One line per example on success, naming every listed refusal with
//! the writer's reason; exit 1 naming the first difference, refusal or failed program otherwise,
//! 2 on a usage error.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codespec_conformance::Calls;
use codespec_conformance::conform::{Process, Program};
use codespec_eval::diff::{self, Plan};
use codespec_model::Module;

const LANGS: &[&str] = &["cir", "go", "rust"];

const USAGE: &str = "usage: codespec-eval-diff --adapters <dir> --build-dir <dir> [--seeds <n>] \
                     [--refused <example>:<lang>:<function>]... <example dir>...";

fn main() -> ExitCode {
    let mut adapters = None;
    let mut build = None;
    let mut seeds = 200u64;
    let mut refused: Vec<(String, String, String)> = Vec::new();
    let mut examples = Vec::new();
    let mut args = std::env::args().skip(1);
    let usage = || {
        eprintln!("{USAGE}");
        ExitCode::from(2)
    };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--adapters" => adapters = args.next().map(PathBuf::from),
            "--build-dir" => build = args.next().map(PathBuf::from),
            "--seeds" => match args.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => seeds = n,
                _ => return usage(),
            },
            "--refused" => {
                let Some(v) = args.next() else {
                    return usage();
                };
                let parts: Vec<&str> = v.splitn(3, ':').collect();
                let [x, lang, func] = parts.as_slice() else {
                    return usage();
                };
                refused.push((x.to_string(), lang.to_string(), func.to_string()));
            }
            _ if a.starts_with('-') => return usage(),
            _ => examples.push(PathBuf::from(a)),
        }
    }
    let (Some(adapters), Some(build)) = (adapters, build) else {
        return usage();
    };
    if examples.is_empty() {
        return usage();
    }
    for x in &examples {
        match example(x, &adapters, &build, seeds, &refused) {
            Ok(line) => println!("{line}"),
            Err(e) => {
                eprintln!("codespec-eval-diff: {}: {e}", x.display());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn absolute(p: &Path) -> Result<PathBuf, String> {
    std::path::absolute(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn put(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn first_lines(s: &str) -> String {
    s.trim().lines().take(6).collect::<Vec<_>>().join(" / ")
}

/// Write the probe module as `lang` into `<dir>/out`: the names it refuses, each with its reason.
fn write(
    lang: &str,
    module: &str,
    adapters: &Path,
    dir: &Path,
) -> Result<Vec<(String, String)>, String> {
    let writer = Process(adapters.join(format!("codespec-write-{lang}")));
    let out = dir.join("out");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let w = writer.exec(&[out.as_os_str().to_owned()], Some(module.as_bytes()))?;
    let listed = std::fs::read_to_string(out.join("refusals.json")).unwrap_or_default();
    let refusals: Vec<(String, String)> = match serde_json::from_str::<serde_json::Value>(&listed) {
        Ok(v) => v
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| {
                let s = |k: &str| r[k].as_str().unwrap_or("").to_string();
                (s("name"), s("reason"))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    if w.code != Some(0) && refusals.is_empty() {
        return Err(format!(
            "codespec-write-{lang} exited {:?}: {}",
            w.code,
            first_lines(&w.stderr)
        ));
    }
    Ok(refusals)
}

/// Run `calls` on what `lang` wrote into `<dir>/out`: the runner's standard output.
fn run(lang: &str, calls: &Path, adapters: &Path, dir: &Path) -> Result<String, String> {
    let runner = Process(adapters.join(format!("codespec-run-{lang}")));
    let args: Vec<OsString> = vec![
        "--build".into(),
        dir.join("build").into_os_string(),
        dir.join("out").into_os_string(),
        calls.as_os_str().to_owned(),
    ];
    let r = runner.exec(&args, None)?;
    if r.code != Some(0) {
        return Err(format!(
            "codespec-run-{lang} exited {:?}: {}",
            r.code,
            first_lines(&r.stderr)
        ));
    }
    String::from_utf8(r.stdout).map_err(|e| format!("codespec-run-{lang}: {e}"))
}

/// The entry a writer's refusal names: by its probe or its own name.
fn entry_of<'p>(plan: &'p Plan, name: &str) -> Option<&'p diff::Entry> {
    plan.entries
        .iter()
        .find(|e| e.call == name || e.func == name)
}

fn example(
    x: &Path,
    adapters: &Path,
    build: &Path,
    seeds: u64,
    listed: &[(String, String, String)],
) -> Result<String, String> {
    let adapters = absolute(adapters)?;
    let name = x
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("not a directory name")?
        .to_string();
    let path = x.join(format!("{name}.cir.json"));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let m = Module::from_json(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let plan = diff::plan(&m)?;
    if !plan.skipped.is_empty() {
        let why: Vec<String> = plan
            .skipped
            .iter()
            .map(|(f, w)| format!("{f}: {w}"))
            .collect();
        return Err(format!(
            "exported functions without an entry: {}",
            why.join("; ")
        ));
    }
    let calls = diff::calls(&plan, seeds);
    let dir = absolute(&build.join(&name))?;
    let module = plan.module.to_json();
    put(&dir.join("probes.cir.json"), &module)?;
    put(&dir.join("calls.json"), &diff::calls_json(&calls))?;

    let mut reference: Option<String> = None;
    let mut notes = Vec::new();
    for lang in LANGS {
        let ldir = dir.join(lang);
        let mut skip: BTreeSet<String> = BTreeSet::new();
        for (fname, reason) in write(lang, &module, &adapters, &ldir)? {
            let e = entry_of(&plan, &fname)
                .ok_or_else(|| format!("codespec-write-{lang} refused {fname}: {reason}"))?;
            if !listed
                .iter()
                .any(|(lx, ll, lf)| *lx == name && ll == lang && *lf == e.func)
            {
                return Err(format!(
                    "codespec-write-{lang} refused {} ({fname}), which --refused does not list: {reason}",
                    e.func
                ));
            }
            notes.push(format!(
                "{} not run as {lang}, whose writer refuses it: {reason}",
                e.func
            ));
            skip.insert(e.call.clone());
        }
        for (lx, ll, lf) in listed {
            if *lx == name && ll == lang {
                let e = plan
                    .entries
                    .iter()
                    .find(|e| e.func == *lf)
                    .ok_or_else(|| format!("--refused {lx}:{ll}:{lf}: no such entry"))?;
                if !skip.contains(&e.call) {
                    return Err(format!(
                        "--refused {lx}:{ll}:{lf}, but codespec-write-{lang} writes it: remove the listing"
                    ));
                }
            }
        }
        if *lang == "cir" && !skip.is_empty() {
            return Err(
                "the evaluator's writer refuses an entry; its output is the reference".into(),
            );
        }
        let kept = Calls {
            format: calls.format.clone(),
            calls: calls
                .calls
                .iter()
                .filter(|c| !skip.contains(&c.func))
                .cloned()
                .collect(),
        };
        let calls_path = if skip.is_empty() {
            dir.join("calls.json")
        } else {
            let p = dir.join(format!("calls-{lang}.json"));
            put(&p, &diff::calls_json(&kept))?;
            p
        };
        let out = run(lang, &calls_path, &adapters, &ldir)?;
        match &reference {
            None => {
                if out.lines().count() != calls.calls.len() {
                    return Err(format!(
                        "the evaluator printed {} lines for {} calls",
                        out.lines().count(),
                        calls.calls.len()
                    ));
                }
                reference = Some(out);
            }
            Some(base) => {
                let base_kept: String = base
                    .lines()
                    .zip(&calls.calls)
                    .filter(|(_, c)| !skip.contains(&c.func))
                    .map(|(l, _)| format!("{l}\n"))
                    .collect();
                diff::compare(&plan, &kept, &[("cir", &base_kept), (lang, &out)])?;
            }
        }
    }
    let probes = plan.entries.iter().filter(|e| e.call != e.func).count();
    let mut line = format!(
        "eval-diff: {name}: {} exported functions, {} called directly, {} through a probe, constructor {} in every method probe; {seeds} seeds each, {} calls: {} byte-equal",
        plan.entries.len() + plan.constructors.len(),
        plan.entries.len() - probes,
        probes,
        plan.constructors.join(", "),
        calls.calls.len(),
        LANGS.join(", ")
    );
    for n in notes {
        line.push_str("; ");
        line.push_str(&n);
    }
    Ok(line)
}
