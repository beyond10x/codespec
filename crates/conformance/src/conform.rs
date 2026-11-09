//! `codespec conform`: run the kit against one adapter and write a
//! `codespec-conformance-report/1` (`docs/conformance.md` §§ 6–7).
//!
//! # The adapter and its programs
//!
//! Every program is run through [`Program`]; [`Process`] runs one by path. The claim is the
//! adapter's `--describe` manifest, read strictly (`codespec_model::manifest`) and expanded against
//! the kit's `kit.json` by [`claim`].
//!
//! | role | programs | `--adapters` names |
//! |---|---|---|
//! | writer | the writer, the runner of its language | `codespec-write-<lang>`, `codespec-run-<lang>` |
//! | reader | the reader, the runner of its language, the trusted Go writer and Go runner | `codespec-read-<lang>`, `codespec-run-<lang>`, `codespec-write-go`, `codespec-run-go` |
//!
//! A writer is called `<writer> <out dir>` with the module on standard input; a runner
//! `<runner> --build <dir> <written dir> <calls.json>`; a reader `<reader> <source dir>`, printing
//! the module. A writer **refuses** a function by exiting non-zero (nothing of the module is
//! written) or by writing `<out dir>/refusals.json`, an array that is not empty; the case's
//! `detail` is then its stderr, or each listed refusal as `name (pos): reason`, joined by `; `. A
//! runner exits 0 with the output, 2 when it refuses the calls, 1 when the build or the program
//! fails.
//!
//! # The run's inputs and its work directory
//!
//! Before any adapter program runs, the run reads every case's `case.cir.json`, `calls.json` and
//! `expected.txt` (a reader's `calls.json` from its corpus), and takes the kit's and the corpus's
//! digests; what a program later does to those files changes neither the comparison nor the
//! report. Programs get absolute paths under the work directory, which is emptied first and named
//! by its canonical path; a runner is given the run's copy of `calls.json`
//! (`<work>/cases/<case>/calls.json`), never the kit's, beside which lies `expected.txt`. A work
//! directory that is, holds or lies inside the adapters directory, the kit or the corpus is
//! refused ([`check_work`]).
//!
//! Every program runs within a time limit, [`DEFAULT_TIMEOUT`] unless the caller gives one
//! (`*_within`, `codespec conform --timeout`); a [`Process`] that overruns is killed with its
//! process group, and the case records `timeout`.
//!
//! # A writer
//!
//! Each case of `<kit>/cases` is written twice from its `case.cir.json` (into `out-1` and
//! `out-2` under `<work>/cases/<case>`), and `out-1` is run on the case's `calls.json`. The case's
//! outcome is *refused*, *pass* (the runner printed `expected.txt`) or *fail*: the output differs
//! (`wrong_output`, with the first differing line), the runner failed (`run_failed`) or refused
//! (`run_refused`), or a program ran past the limit (`timeout`). The boundary cases, in this
//! order:
//!
//! | id | module | passes when |
//! |---|---|---|
//! | `runner_control` | none: the runner is given an empty directory and the first case's calls | it does not print that case's `expected.txt`, so its output comes from what the writer wrote |
//! | `unknown_kind` | the first case's module with a statement of kind `codespec_conformance_unknown` first in its first function | exit non-zero, nothing written |
//! | `no_facts` | the first case declaring a `fact.` feature (else the first case), without `facts` | exit non-zero, nothing written, the message names `codespec pass`; or written and run to `expected.txt`, for a writer that does not read facts |
//! | `deterministic` | every case | both writes give the same exit status and the same files, byte for byte |
//!
//! # A reader
//!
//! The corpus (`--corpus`, default `<kit>/reader/<lang>`) holds one directory per case,
//! named as the kit case whose subject it covers: the source in the reader's language and its
//! `calls.json`, read as `read_calls` reads one. A directory naming no kit case is refused. Per
//! corpus case, the steps of § 6, the first failing one recorded as `step`:
//!
//! 1. the runner of the reader's language runs the source as written and prints the kit case's
//!    `expected.txt`;
//! 2. the reader reads it: exit 0, a valid `cir/1` module, the same bytes on a second read
//!    (`read_failed`, `invalid_module`, `not_deterministic`);
//! 3. the module uses the case's subject outside a `foreign` statement, as
//!    [`crate::features_used`] names it (`subject_missing`), and holds no `foreign` statement at
//!    all (`foreign_statement`), unless the subject is `stmt.foreign`: kept source can hide the
//!    construct behind a dead copy of it, so a case counts only when nothing of it is foreign;
//! 4. `codespec pass` (`codespec_passes::run`) and the Go writer write it (`refused` when that
//!    writer refuses), and the Go runner prints `expected.txt` with the corpus case's `calls.json`.
//!
//! An unclaimed feature whose case stops at step 2 by `read_failed` or at step 3 is *refused*;
//! a claimed one fails.
//!
//! ## The in-repository corpora
//!
//! The corpus of each reader this repository ships, `conformance/reader/<lang>` for a language of
//! [`CORPUS_LANGS`], is built by [`reader_corpus`] from the kit's own case sources, never written
//! by hand nor copied from a writer's output; `codespec-docs generate` writes it and `generate
//! --check` fails when a file differs from its case source, is missing or is extra. Per kit case:
//!
//! | lang | from the case | corpus files |
//! |---|---|---|
//! | `go` | `case.go` | `case.go` |
//! | `rust` | `rust/case.rs` | `src/lib.rs` and a `Cargo.toml` naming the package `<case>` with `-` read as `_`, with its own `[workspace]` |
//! | `java` | `java/Case.java` | `Case.java` |
//! | `groovy` | `groovy/Case.groovy`, else `java/Case.java` | `Case.groovy`: Groovy 2.4 compiles every Java source of the kit unchanged, and `codespec-run-groovy` runs it to the case's `expected.txt` at step 1; a case whose Java source means something else in Groovy (`/` on integers, a BigDecimal division there) carries its own `groovy/Case.groovy` |
//! | `c` | `c/case.c` | `case.c` |
//!
//! each beside a copy of the case's `calls.json`. A case without the source has no corpus entry.
//!
//! # Verdicts
//!
//! A feature's cases are those whose subject it is (the kit's cases for a writer, the corpus's for a
//! reader). Its verdict:
//!
//! | claimed | no case | every case passes | a case refused, none failed | a case failed |
//! |---|---|---|---|---|
//! | yes | `fail`, `no_case` | `pass` | `fail`, `refused` | `fail` |
//! | no | `untested` | `pass` | `refused` | `fail` |
//!
//! `case`, `step`, `reason`, `first_difference` and `detail` describe the first case, by name,
//! that did not pass. `conforms` is true when no feature is `fail` and no boundary case is `fail`.
//!
//! # The report: `codespec-conformance-report/1`
//!
//! One JSON object, two-space indentation, a final newline ([`report_json`]); [`read_report`]
//! reads it strictly.
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-conformance-report/1` |
//! | `adapter` | object | `role`, `lang`, `version` from the manifest; `manifest_sha256`, the SHA-256 of the manifest's bytes |
//! | `kit` | object | `version`, the codespec version the kit ships with; `sha256`, [`tree_sha256`] of `kit.json` and `cases/` |
//! | `corpus` | object, readers only | `sha256`, [`tree_sha256`] of the corpus |
//! | `codespec` | string | the version of the codespec that ran it |
//! | `features` | array | one entry per feature of `kit.json`, in its order (below) |
//! | `boundary` | array | for a writer, the four boundary cases above: `id`, `verdict` (`pass` or `fail`), `case` and `detail` when it failed; empty for a reader |
//! | `summary` | object | counts: `claimed`; `pass`, claimed and passing; `fail`; `refused_unclaimed`; `pass_unclaimed`; `untested`; `boundary_fail` |
//! | `conforms` | bool | as above |
//!
//! A feature entry:
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `id` | string | the feature id |
//! | `claimed` | bool | whether the manifest's expanded claim holds it |
//! | `verdict` | string | `pass`, `fail`, `refused` or `untested` |
//! | `cases` | array of string | the cases run for it |
//! | `case` | string, optional | the first case that did not pass |
//! | `step` | number, optional | for a reader, the step of § 6 it stopped at, 1 to 4 |
//! | `reason` | string, optional | `no_case`, `refused`, `wrong_output`, `run_failed`, `run_refused`, `read_failed`, `invalid_module`, `not_deterministic`, `subject_missing`, `foreign_statement` or `timeout` |
//! | `first_difference` | object, optional | `line` (1-based), `expected`, `got`, for `wrong_output` |
//! | `detail` | string, optional | the program's message, its first lines, with the work, kit and corpus directories written as `<work>`, `<kit>`, `<corpus>` |
//!
//! A cell of the support matrix for one feature and one adapter follows from `claimed` and
//! `verdict`: claimed and `pass` is verified; `fail` is failing; unclaimed and `refused` or
//! `untested` is refused; unclaimed and `pass` works without a claim.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use codespec_model::{AdapterManifest, Module};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    Case, Error, KIT_FORMAT, Kit, features_used, first_difference, load_cases, read_calls,
};

/// The format of a conformance report.
pub const REPORT_FORMAT: &str = "codespec-conformance-report/1";

/// The codespec version: the `codespec` of every report and the `version` of the kit it ships.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The kind of the statement the `unknown_kind` boundary case puts into a module.
pub const UNKNOWN_KIND: &str = "codespec_conformance_unknown";

/// What a program did: its exit code (`None` when a signal ended it), standard output and error.
#[derive(Debug, Clone)]
pub struct Exec {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// How long one program may run unless the caller says otherwise: 300 seconds.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// How many bytes of a program's stdout, and of its stderr, are kept: 64 MiB. A program that
/// writes more loses its pipe, so one that prints without end cannot fill memory.
pub const OUTPUT_LIMIT: u64 = 64 << 20;

/// What came of running a program within a time limit.
#[derive(Debug, Clone)]
pub enum Ran {
    Done(Exec),
    /// It ran past the limit; a [`Process`] is killed with its process group.
    TimedOut,
}

/// A program the run calls: an adapter, a runner, or in a test a stub.
pub trait Program: Sync {
    /// Run with `args`, `stdin` on standard input when given. An error means it could not start.
    fn exec(&self, args: &[OsString], stdin: Option<&[u8]>) -> Result<Exec, Error>;

    /// Run as [`Program::exec`] does, within `limit`. This default cannot stop a program that
    /// overruns: it waits, then reports [`Ran::TimedOut`]. [`Process`] stops it at the limit.
    fn exec_within(
        &self,
        args: &[OsString],
        stdin: Option<&[u8]>,
        limit: Duration,
    ) -> Result<Ran, Error> {
        let start = Instant::now();
        let e = self.exec(args, stdin)?;
        Ok(if start.elapsed() > limit {
            Ran::TimedOut
        } else {
            Ran::Done(e)
        })
    }
}

impl<F> Program for F
where
    F: Fn(&[OsString], Option<&[u8]>) -> Result<Exec, Error> + Sync,
{
    fn exec(&self, args: &[OsString], stdin: Option<&[u8]>) -> Result<Exec, Error> {
        self(args, stdin)
    }
}

/// A program on disk.
#[derive(Debug, Clone)]
pub struct Process(pub PathBuf);

impl Program for Process {
    fn exec(&self, args: &[OsString], stdin: Option<&[u8]>) -> Result<Exec, Error> {
        match self.exec_within(args, stdin, Duration::MAX)? {
            Ran::Done(e) => Ok(e),
            Ran::TimedOut => Err(format!("{}: did not finish", self.0.display())),
        }
    }

    /// Run in its own process group, killing the whole group (the program, its compiler, the
    /// program it built) once `limit` has passed.
    fn exec_within(
        &self,
        args: &[OsString],
        stdin: Option<&[u8]>,
        limit: Duration,
    ) -> Result<Ran, Error> {
        let what = || self.0.display().to_string();
        let mut cmd = Command::new(&self.0);
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("{}: cannot start: {e}", what()))?;
        let feeder = stdin.map(|bytes| {
            let mut pipe = child.stdin.take().expect("stdin is piped");
            let bytes = bytes.to_vec();
            // A program may exit before it reads everything; what it did is in its status.
            std::thread::spawn(move || {
                let _ = pipe.write_all(&bytes);
            })
        });
        let drain = |r: Option<Box<dyn Read + Send>>| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(r) = r {
                    let _ = r.take(OUTPUT_LIMIT).read_to_end(&mut buf);
                }
                buf
            })
        };
        let out = drain(
            child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn Read + Send>),
        );
        let err = drain(
            child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn Read + Send>),
        );
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(|e| format!("{}: {e}", what()))? {
                break Some(s);
            }
            if start.elapsed() >= limit {
                kill_group(&mut child);
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let stdout = out.join().unwrap_or_default();
        let stderr = err.join().unwrap_or_default();
        if let Some(f) = feeder {
            let _ = f.join();
        }
        Ok(match status {
            None => Ran::TimedOut,
            Some(s) => Ran::Done(Exec {
                code: s.code(),
                stdout,
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            }),
        })
    }
}

/// Kill a child started in its own process group, and the group, then reap it.
fn kill_group(child: &mut std::process::Child) {
    // Signal the group directly: a host may have no `kill` program (the GitHub Ubuntu runner has
    // none) and shells disagree on `kill`'s arguments (dash reads `--` as a pid), and then only
    // the child would die while its children keep its pipes open.
    #[cfg(unix)]
    if let Ok(group) = libc::pid_t::try_from(child.id()) {
        // SAFETY: kill(2) takes plain integers and touches no memory of this process; the group
        // is the one the child leads, started with `process_group(0)`.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

impl Process {
    /// The manifest the program prints for `--describe`; it must exit 0 within
    /// [`DEFAULT_TIMEOUT`].
    pub fn describe(&self) -> Result<Vec<u8>, Error> {
        let e = match self.exec_within(&["--describe".into()], None, DEFAULT_TIMEOUT)? {
            Ran::Done(e) => e,
            Ran::TimedOut => {
                return Err(format!(
                    "{} --describe ran past {} s",
                    self.0.display(),
                    DEFAULT_TIMEOUT.as_secs()
                ));
            }
        };
        if e.code != Some(0) {
            return Err(format!(
                "{} --describe exited with {:?}: {}",
                self.0.display(),
                e.code,
                e.stderr.trim()
            ));
        }
        Ok(e.stdout)
    }
}

/// One conformance run: the adapter's manifest bytes, the kit directory, and the directory the
/// run works in (emptied first).
#[derive(Debug, Clone)]
pub struct Run<'a> {
    pub manifest: &'a [u8],
    pub kit: PathBuf,
    pub work: PathBuf,
}

/// A verdict on a feature or a boundary case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Refused,
    Untested,
}

/// Why a case did not pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NoCase,
    Refused,
    WrongOutput,
    RunFailed,
    RunRefused,
    ReadFailed,
    InvalidModule,
    NotDeterministic,
    SubjectMissing,
    ForeignStatement,
    Timeout,
}

impl std::fmt::Display for Reason {
    /// The reason as the report spells it: `no_case`, `wrong_output`, ….
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = serde_json::to_value(self).expect("a reason serializes");
        f.write_str(v.as_str().expect("a reason is a string"))
    }
}

impl std::fmt::Display for Verdict {
    /// The verdict as the report spells it: `pass`, `fail`, `refused`, `untested`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = serde_json::to_value(self).expect("a verdict serializes");
        f.write_str(v.as_str().expect("a verdict is a string"))
    }
}

/// Where two outputs first differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirstDifference {
    pub line: usize,
    pub expected: String,
    pub got: String,
}

/// The `adapter` of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterRef {
    pub role: String,
    pub lang: String,
    pub version: String,
    pub manifest_sha256: String,
}

/// The `kit` of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitRef {
    pub version: String,
    pub sha256: String,
}

/// The `corpus` of a reader's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusRef {
    pub sha256: String,
}

/// One entry of `features`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureResult {
    pub id: String,
    pub claimed: bool,
    pub verdict: Verdict,
    pub cases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_difference: Option<FirstDifference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// One entry of `boundary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryResult {
    pub id: String,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// The `summary` of a report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub claimed: usize,
    pub pass: usize,
    pub fail: usize,
    pub refused_unclaimed: usize,
    pub pass_unclaimed: usize,
    pub untested: usize,
    pub boundary_fail: usize,
}

/// A `codespec-conformance-report/1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    pub adapter: AdapterRef,
    pub kit: KitRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpus: Option<CorpusRef>,
    pub codespec: String,
    pub features: Vec<FeatureResult>,
    pub boundary: Vec<BoundaryResult>,
    pub summary: Summary,
    pub conforms: bool,
}

/// The canonical encoding of a report: two-space indentation, a final newline.
pub fn report_json(r: &Report) -> String {
    let mut s = serde_json::to_string_pretty(r).expect("a report always serializes");
    s.push('\n');
    s
}

/// Read a report strictly: unknown fields, another format and an encoding other than
/// [`report_json`] are refused.
pub fn read_report(bytes: &[u8]) -> Result<Report, Error> {
    let probe: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let found = probe.get("format").and_then(Value::as_str).unwrap_or("");
    if found != REPORT_FORMAT {
        return Err(format!(
            "format is {found:?}, this reader reads {REPORT_FORMAT:?}"
        ));
    }
    let r: Report = serde_json::from_value(probe).map_err(|e| e.to_string())?;
    if report_json(&r).as_bytes() != bytes {
        return Err("not canonical (two-space indentation, a final newline)".into());
    }
    Ok(r)
}

/// Read `<kit>/kit.json` strictly.
pub fn read_kit(kit: &Path) -> Result<Kit, Error> {
    let path = kit.join("kit.json");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let probe: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let found = probe.get("format").and_then(Value::as_str).unwrap_or("");
    if found != KIT_FORMAT {
        return Err(format!(
            "{}: format is {found:?}, this reader reads {KIT_FORMAT:?}",
            path.display()
        ));
    }
    serde_json::from_value(probe).map_err(|e| format!("{}: {e}", path.display()))
}

/// The manifest read strictly and its claim expanded against the kit's features.
pub fn claim(manifest: &[u8], kit: &Kit) -> Result<(AdapterManifest, BTreeSet<String>), Error> {
    let m = codespec_model::manifest_from_json(manifest).map_err(|e| format!("manifest: {e}"))?;
    let claimed = m
        .claimed_in(&kit.features)
        .map_err(|e| format!("manifest against the kit: {e}"))?;
    Ok((m, claimed))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn files_under(dir: &Path, rel: &str, out: &mut BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let path = e.path();
        if path.is_dir() {
            files_under(&path, &rel, out)?;
        } else {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            out.insert(rel, bytes);
        }
    }
    Ok(())
}

/// The SHA-256, in lower-case hex, of the files under `dir` named by `roots` (files or
/// directories, relative to `dir`): for each file in order of its `/`-separated relative path, the
/// path, a newline, its length in decimal, a newline and its bytes.
pub fn tree_sha256(dir: &Path, roots: &[&str]) -> Result<String, Error> {
    let mut files = BTreeMap::new();
    for r in roots {
        let p = dir.join(r);
        if p.is_dir() {
            files_under(&p, r, &mut files)?;
        } else {
            let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            files.insert((*r).to_string(), bytes);
        }
    }
    let mut h = Sha256::new();
    for (path, bytes) in files {
        h.update(format!("{path}\n{}\n", bytes.len()).as_bytes());
        h.update(&bytes);
    }
    Ok(hex(&h.finalize()))
}

/// The outcome of one case.
#[derive(Debug, Clone)]
enum Outcome {
    Pass,
    /// Did not pass; `refusal` when the adapter declined it rather than got it wrong.
    NotPass {
        refusal: bool,
        step: Option<u8>,
        reason: Reason,
        first_difference: Option<FirstDifference>,
        detail: Option<String>,
    },
}

/// Text that names none of the directories of the run.
struct Clean {
    swaps: Vec<(String, &'static str)>,
}

impl Clean {
    /// Each directory is replaced as given, as an absolute path and as its canonical path.
    fn new(dirs: &[(&Path, &'static str)]) -> Clean {
        let mut swaps = Vec::new();
        for (dir, name) in dirs {
            let mut forms = vec![dir.to_path_buf()];
            if let Ok(a) = std::path::absolute(dir) {
                forms.push(a);
            }
            if let Ok(c) = dir.canonicalize() {
                forms.push(c);
            }
            for f in forms {
                let s = f.display().to_string();
                if !s.is_empty() && s != "." && !swaps.iter().any(|(x, _)| x == &s) {
                    swaps.push((s, *name));
                }
            }
        }
        swaps.sort_by_key(|(s, _)| std::cmp::Reverse(s.len()));
        Clean { swaps }
    }

    fn text(&self, s: &str) -> Option<String> {
        let mut t: String = s.trim().lines().take(4).collect::<Vec<_>>().join(" / ");
        for (from, to) in &self.swaps {
            t = t.replace(from.as_str(), to);
        }
        if t.chars().count() > 400 {
            t = t.chars().take(400).collect::<String>() + "…";
        }
        (!t.is_empty()).then_some(t)
    }
}

/// What every program call of one run shares: the time limit and the text cleaner.
struct Ctx {
    clean: Clean,
    limit: Duration,
}

impl Ctx {
    fn exec(&self, p: &dyn Program, args: &[OsString], stdin: Option<&[u8]>) -> Result<Ran, Error> {
        p.exec_within(args, stdin, self.limit)
    }

    fn overran(&self, what: &str) -> String {
        format!("{what} ran past the limit of {} s", self.limit.as_secs())
    }
}

fn not_pass(reason: Reason, detail: Option<String>) -> Outcome {
    Outcome::NotPass {
        refusal: false,
        step: None,
        reason,
        first_difference: None,
        detail,
    }
}

fn at_step(o: Outcome, n: u8) -> Outcome {
    match o {
        Outcome::NotPass {
            refusal,
            reason,
            first_difference,
            detail,
            ..
        } => Outcome::NotPass {
            refusal,
            step: Some(n),
            reason,
            first_difference,
            detail,
        },
        pass => pass,
    }
}

fn reset(dir: &Path) -> Result<(), Error> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

fn mkdir(dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

fn put(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        mkdir(parent)?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The files a writer wrote, or `None` when it wrote nothing.
fn snapshot(dir: &Path) -> Result<Option<BTreeMap<String, Vec<u8>>>, Error> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut out = BTreeMap::new();
    files_under(dir, "", &mut out)?;
    Ok((!out.is_empty()).then_some(out))
}

/// What a writer did with a module.
enum Wrote {
    Refused(String),
    Done,
    TimedOut,
}

/// Write `module` into `out`; also the writer's exit code, `None` when it timed out.
/// A writer's `refusals.json` as one line, each refusal `name (pos): reason` (`pos` when given),
/// joined by `; `; `None` when it is not an array of objects each with a string `name` and
/// `reason`, and then the file's text is the detail.
fn refusal_lines(listed: &Value) -> Option<String> {
    let mut out = Vec::new();
    for r in listed.as_array()? {
        let name = r.get("name")?.as_str()?;
        let reason = r.get("reason")?.as_str()?;
        match r.get("pos").and_then(Value::as_str) {
            Some(pos) => out.push(format!("{name} ({pos}): {reason}")),
            None => out.push(format!("{name}: {reason}")),
        }
    }
    Some(out.join("; "))
}

fn write(
    ctx: &Ctx,
    writer: &dyn Program,
    module: &[u8],
    out: &Path,
) -> Result<(Wrote, Option<i32>), Error> {
    reset(out)?;
    let e = match ctx.exec(writer, &[out.as_os_str().to_owned()], Some(module))? {
        Ran::Done(e) => e,
        Ran::TimedOut => return Ok((Wrote::TimedOut, None)),
    };
    if e.code != Some(0) {
        return Ok((Wrote::Refused(e.stderr), e.code));
    }
    let refusals = out.join("refusals.json");
    if refusals.is_file() {
        let text = std::fs::read_to_string(&refusals)
            .map_err(|e| format!("{}: {e}", refusals.display()))?;
        let listed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if !listed.as_array().is_some_and(|a| a.is_empty()) {
            return Ok((
                Wrote::Refused(refusal_lines(&listed).unwrap_or(text)),
                e.code,
            ));
        }
    }
    Ok((Wrote::Done, e.code))
}

/// Run `dir` on `calls` and compare with `expected`.
fn run_and_compare(
    ctx: &Ctx,
    runner: &dyn Program,
    build: &Path,
    dir: &Path,
    calls: &Path,
    expected: &str,
) -> Result<Outcome, Error> {
    let args: Vec<OsString> = vec![
        "--build".into(),
        build.as_os_str().to_owned(),
        dir.as_os_str().to_owned(),
        calls.as_os_str().to_owned(),
    ];
    let e = match ctx.exec(runner, &args, None)? {
        Ran::Done(e) => e,
        Ran::TimedOut => return Ok(not_pass(Reason::Timeout, Some(ctx.overran("the runner")))),
    };
    match e.code {
        Some(0) => {
            let got = String::from_utf8_lossy(&e.stdout);
            Ok(match first_difference(expected, &got) {
                None => Outcome::Pass,
                Some(d) => Outcome::NotPass {
                    refusal: false,
                    step: None,
                    reason: Reason::WrongOutput,
                    first_difference: Some(FirstDifference {
                        line: d.line,
                        expected: d.expected,
                        got: d.got,
                    }),
                    detail: None,
                },
            })
        }
        Some(2) => Ok(not_pass(Reason::RunRefused, ctx.clean.text(&e.stderr))),
        _ => Ok(not_pass(Reason::RunFailed, ctx.clean.text(&e.stderr))),
    }
}

/// A kit case with the inputs the run compares against, read before any adapter program runs.
struct Loaded<'a> {
    case: &'a Case,
    module: Vec<u8>,
    expected: String,
    calls: Vec<u8>,
}

fn read_file(p: &Path) -> Result<Vec<u8>, Error> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn load(case: &Case) -> Result<Loaded<'_>, Error> {
    let expected = String::from_utf8(read_file(&case.dir.join("expected.txt"))?)
        .map_err(|e| format!("{}/expected.txt: {e}", case.name))?;
    Ok(Loaded {
        case,
        module: read_file(&case.dir.join("case.cir.json"))?,
        expected,
        calls: read_file(&case.dir.join("calls.json"))?,
    })
}

/// Run `f` over `items` on several threads, keeping input order; the first error wins.
fn parallel<T: Sync, R: Send>(
    items: &[T],
    f: impl Fn(&T) -> Result<R, Error> + Sync,
) -> Result<Vec<R>, Error> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<Result<R, Error>>>> =
        Mutex::new((0..items.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    out.lock().expect("no worker panics")[i] = Some(r);
                }
            });
        }
    });
    out.into_inner()
        .expect("no worker panics")
        .into_iter()
        .map(|r| r.expect("every item ran"))
        .collect()
}

fn verdicts(
    kit: &Kit,
    claimed: &BTreeSet<String>,
    ran: &[(String, String, Outcome)],
) -> Vec<FeatureResult> {
    kit.features
        .iter()
        .map(|f| {
            let is_claimed = claimed.contains(&f.id);
            let mine: Vec<&(String, String, Outcome)> =
                ran.iter().filter(|(_, s, _)| s == &f.id).collect();
            let mut r = FeatureResult {
                id: f.id.clone(),
                claimed: is_claimed,
                verdict: Verdict::Pass,
                cases: mine.iter().map(|(c, _, _)| c.clone()).collect(),
                case: None,
                step: None,
                reason: None,
                first_difference: None,
                detail: None,
            };
            if mine.is_empty() {
                if is_claimed {
                    r.verdict = Verdict::Fail;
                    r.reason = Some(Reason::NoCase);
                } else {
                    r.verdict = Verdict::Untested;
                }
                return r;
            }
            let failed = mine
                .iter()
                .find(|(_, _, o)| matches!(o, Outcome::NotPass { refusal: false, .. }));
            let refused = mine
                .iter()
                .find(|(_, _, o)| matches!(o, Outcome::NotPass { refusal: true, .. }));
            let first = failed.or(refused);
            if let Some((
                case,
                _,
                Outcome::NotPass {
                    step,
                    reason,
                    first_difference,
                    detail,
                    ..
                },
            )) = first
            {
                r.verdict = if failed.is_some() || is_claimed {
                    Verdict::Fail
                } else {
                    Verdict::Refused
                };
                r.case = Some(case.clone());
                r.step = *step;
                r.reason = Some(*reason);
                r.first_difference = first_difference.clone();
                r.detail = detail.clone();
            }
            r
        })
        .collect()
}

fn finish(
    m: &AdapterManifest,
    manifest: &[u8],
    kit_sha: String,
    corpus: Option<CorpusRef>,
    features: Vec<FeatureResult>,
    boundary: Vec<BoundaryResult>,
) -> Report {
    let mut s = Summary::default();
    for f in &features {
        match (f.claimed, f.verdict) {
            (true, Verdict::Pass) => s.pass += 1,
            (false, Verdict::Pass) => s.pass_unclaimed += 1,
            (_, Verdict::Fail) => s.fail += 1,
            (_, Verdict::Refused) => s.refused_unclaimed += 1,
            (_, Verdict::Untested) => s.untested += 1,
        }
        if f.claimed {
            s.claimed += 1;
        }
    }
    s.boundary_fail = boundary
        .iter()
        .filter(|b| b.verdict == Verdict::Fail)
        .count();
    let conforms = s.fail == 0 && s.boundary_fail == 0;
    Report {
        format: REPORT_FORMAT.into(),
        adapter: AdapterRef {
            role: m.role.clone(),
            lang: m.lang.clone(),
            version: m.version.clone(),
            manifest_sha256: hex(&Sha256::digest(manifest)),
        },
        kit: KitRef {
            version: VERSION.into(),
            sha256: kit_sha,
        },
        corpus,
        codespec: VERSION.into(),
        features,
        boundary,
        summary: s,
        conforms,
    }
}

/// Refuse a work directory that is, holds or lies inside one of the run's inputs: the run empties
/// it first, and the programs write into it.
pub fn check_work(work: &Path, inputs: &[(&str, &Path)]) -> Result<(), Error> {
    let abs = |p: &Path| {
        let a = std::path::absolute(p).map_err(|e| format!("{}: {e}", p.display()))?;
        Ok::<PathBuf, Error>(a.canonicalize().unwrap_or(a))
    };
    let w = abs(work)?;
    for (name, input) in inputs {
        let i = abs(input)?;
        if i.starts_with(&w) {
            return Err(format!(
                "the work directory {} holds the {name} {}; it is emptied before a run",
                work.display(),
                input.display()
            ));
        }
        if w.starts_with(&i) {
            return Err(format!(
                "the work directory {} is inside the {name} {}; the programs write into it",
                work.display(),
                input.display()
            ));
        }
    }
    Ok(())
}

/// Empty the work directory and return its canonical path.
fn clear_work(work: &Path) -> Result<PathBuf, Error> {
    reset(work)?;
    mkdir(work)?;
    work.canonicalize()
        .map_err(|e| format!("{}: {e}", work.display()))
}

/// What a run knows before any adapter program runs.
struct Prepared {
    kit: Kit,
    manifest: AdapterManifest,
    claimed: BTreeSet<String>,
    cases: Vec<Case>,
    kit_sha: String,
    /// The work directory, emptied, canonical.
    work: PathBuf,
}

fn prepare(run: &Run, role: &str, inputs: &[(&str, &Path)]) -> Result<Prepared, Error> {
    let kit = read_kit(&run.kit)?;
    let (m, claimed) = claim(run.manifest, &kit)?;
    if m.role != role {
        return Err(format!(
            "the manifest's role is {:?}, this run checks a {role}",
            m.role
        ));
    }
    let kit_dir = std::path::absolute(&run.kit).map_err(|e| e.to_string())?;
    let cases = load_cases(&kit_dir.join("cases"))?;
    let known: BTreeSet<&str> = kit.features.iter().map(|f| f.id.as_str()).collect();
    for c in &cases {
        if !known.contains(c.subject()) {
            return Err(format!(
                "case {}: subject {} is not in kit.json",
                c.name,
                c.subject()
            ));
        }
    }
    let mut all = vec![("kit", run.kit.as_path())];
    all.extend_from_slice(inputs);
    check_work(&run.work, &all)?;
    let kit_sha = tree_sha256(&run.kit, &["kit.json", "cases"])?;
    let work = clear_work(&run.work)?;
    Ok(Prepared {
        kit,
        manifest: m,
        claimed,
        cases,
        kit_sha,
        work,
    })
}

/// A module with a statement of an unknown kind first in its first function.
fn with_unknown_kind(module: &[u8]) -> Result<Vec<u8>, Error> {
    let mut v: Value = serde_json::from_slice(module).map_err(|e| e.to_string())?;
    let body = v["funcs"][0]["body"]
        .as_array_mut()
        .ok_or("the first case has no function")?;
    body.insert(
        0,
        serde_json::json!({"k": UNKNOWN_KIND, "pos": "case.go:1"}),
    );
    Ok(serde_json::to_vec(&v).expect("JSON serializes"))
}

fn without_facts(module: &[u8]) -> Result<Vec<u8>, Error> {
    let mut v: Value = serde_json::from_slice(module).map_err(|e| e.to_string())?;
    v.as_object_mut()
        .ok_or("a module is an object")?
        .remove("facts");
    Ok(serde_json::to_vec(&v).expect("JSON serializes"))
}

fn boundary(id: &str, fail: Option<(Option<String>, Option<String>)>) -> BoundaryResult {
    match fail {
        None => BoundaryResult {
            id: id.into(),
            verdict: Verdict::Pass,
            case: None,
            detail: None,
        },
        Some((case, detail)) => BoundaryResult {
            id: id.into(),
            verdict: Verdict::Fail,
            case,
            detail,
        },
    }
}

/// Run the kit against a writer and its runner, as the module doc says, each program within
/// [`DEFAULT_TIMEOUT`].
pub fn conform_writer(
    run: &Run,
    writer: &dyn Program,
    runner: &dyn Program,
) -> Result<Report, Error> {
    conform_writer_within(run, DEFAULT_TIMEOUT, writer, runner)
}

/// [`conform_writer`] with each program limited to `limit`.
pub fn conform_writer_within(
    run: &Run,
    limit: Duration,
    writer: &dyn Program,
    runner: &dyn Program,
) -> Result<Report, Error> {
    let p = prepare(run, "writer", &[])?;
    let ctx = Ctx {
        clean: Clean::new(&[(&p.work, "<work>"), (&run.kit, "<kit>")]),
        limit,
    };
    let loaded = p.cases.iter().map(load).collect::<Result<Vec<_>, _>>()?;
    let calls_of = |l: &Loaded| p.work.join("cases").join(&l.case.name).join("calls.json");
    for l in &loaded {
        put(&calls_of(l), &l.calls)?;
    }
    let first = loaded.first().ok_or("the kit has no cases")?;
    let dir = p.work.join("boundary");
    let mut bounds = Vec::new();

    let empty = dir.join("runner_control/empty");
    mkdir(&empty)?;
    let control = run_and_compare(
        &ctx,
        runner,
        &dir.join("runner_control/build"),
        &empty,
        &calls_of(first),
        &first.expected,
    )?;
    bounds.push(boundary(
        "runner_control",
        matches!(control, Outcome::Pass).then(|| {
            (
                Some(first.case.name.clone()),
                Some(
                    "the runner printed the case's expected output for an empty directory: \
                     its output does not come from what the writer wrote"
                        .to_string(),
                ),
            )
        }),
    ));

    // Each case: (name, subject, outcome, both writes alike).
    let ran = parallel(&loaded, |l| {
        let case_dir = p.work.join("cases").join(&l.case.name);
        let (one, c1) = write(&ctx, writer, &l.module, &case_dir.join("out-1"))?;
        let (_, c2) = write(&ctx, writer, &l.module, &case_dir.join("out-2"))?;
        let same =
            c1 == c2 && snapshot(&case_dir.join("out-1"))? == snapshot(&case_dir.join("out-2"))?;
        let outcome = match one {
            Wrote::Refused(why) => Outcome::NotPass {
                refusal: true,
                step: None,
                reason: Reason::Refused,
                first_difference: None,
                detail: ctx.clean.text(&why),
            },
            Wrote::TimedOut => not_pass(Reason::Timeout, Some(ctx.overran("the writer"))),
            Wrote::Done => run_and_compare(
                &ctx,
                runner,
                &case_dir.join("build"),
                &case_dir.join("out-1"),
                &calls_of(l),
                &l.expected,
            )?,
        };
        Ok((
            l.case.name.clone(),
            l.case.subject().to_string(),
            outcome,
            same,
        ))
    })?;

    let unknown = with_unknown_kind(&first.module)?;
    let out = dir.join("unknown_kind");
    let (w, code) = write(&ctx, writer, &unknown, &out)?;
    bounds.push(boundary(
        "unknown_kind",
        match (w, snapshot(&out)?) {
            (Wrote::Refused(_), None) if code != Some(0) => None,
            (Wrote::TimedOut, _) => Some((
                Some(first.case.name.clone()),
                Some(ctx.overran("the writer")),
            )),
            _ => Some((
                Some(first.case.name.clone()),
                Some(format!(
                    "exit {code:?}, wrote {}: a module with a statement of kind {UNKNOWN_KIND:?} must be refused with nothing written",
                    if out.exists() { "files" } else { "nothing" }
                )),
            )),
        },
    ));

    let facts_case = loaded
        .iter()
        .find(|l| {
            l.case
                .features
                .features
                .iter()
                .any(|f| f.starts_with("fact."))
        })
        .unwrap_or(first);
    let out = dir.join("no_facts");
    let module = without_facts(&facts_case.module)?;
    let (w, code) = write(&ctx, writer, &module, &out)?;
    let no_facts = match w {
        Wrote::TimedOut => Some(ctx.overran("the writer")),
        Wrote::Refused(why) if code == Some(0) => Some(format!(
            "listed refusals for a module without facts and exited 0: {}",
            ctx.clean.text(&why).unwrap_or_default()
        )),
        Wrote::Refused(stderr) => {
            if snapshot(&out)?.is_some() {
                Some("refused a module without facts, and wrote files".into())
            } else if !stderr.contains("codespec pass") {
                Some(format!(
                    "refused a module without facts without naming `codespec pass`: {}",
                    ctx.clean.text(&stderr).unwrap_or_default()
                ))
            } else {
                None
            }
        }
        Wrote::Done => match run_and_compare(
            &ctx,
            runner,
            &dir.join("no_facts-build"),
            &out,
            &calls_of(facts_case),
            &facts_case.expected,
        )? {
            Outcome::Pass => None,
            Outcome::NotPass {
                reason,
                first_difference,
                detail,
                ..
            } => Some(match first_difference {
                Some(d) => format!(
                    "wrote a module without facts wrong: line {} expected {:?}, got {:?}",
                    d.line, d.expected, d.got
                ),
                None => format!(
                    "wrote a module without facts, and the run ended {reason}: {}",
                    detail.unwrap_or_default()
                ),
            }),
        },
    };
    bounds.push(boundary(
        "no_facts",
        no_facts.map(|d| (Some(facts_case.case.name.clone()), Some(d))),
    ));

    let unstable = ran.iter().find(|r| !r.3);
    bounds.push(boundary(
        "deterministic",
        unstable.map(|r| {
            (
                Some(r.0.clone()),
                Some("written twice, with different exit status or bytes".to_string()),
            )
        }),
    ));

    let ran: Vec<(String, String, Outcome)> = ran.into_iter().map(|r| (r.0, r.1, r.2)).collect();
    let features = verdicts(&p.kit, &p.claimed, &ran);
    Ok(finish(
        &p.manifest,
        run.manifest,
        p.kit_sha,
        None,
        features,
        bounds,
    ))
}

/// The programs a reader run calls.
struct ReaderPrograms<'a> {
    reader: &'a dyn Program,
    runner: &'a dyn Program,
    go_writer: &'a dyn Program,
    go_runner: &'a dyn Program,
}

/// One reader corpus case, through the four steps. `source` is the corpus case's directory,
/// `calls` the run's copy of its `calls.json`.
fn reader_case(
    ctx: &Ctx,
    l: &Loaded,
    source: &Path,
    calls: &Path,
    work: &Path,
    p: &ReaderPrograms,
) -> Result<Outcome, Error> {
    let one = run_and_compare(
        ctx,
        p.runner,
        &work.join("source-build"),
        source,
        calls,
        &l.expected,
    )?;
    if let Outcome::NotPass { .. } = one {
        return Ok(at_step(one, 1));
    }
    let arg = [source.as_os_str().to_owned()];
    let first = match ctx.exec(p.reader, &arg, None)? {
        Ran::Done(e) => e,
        Ran::TimedOut => {
            return Ok(at_step(
                not_pass(Reason::Timeout, Some(ctx.overran("the reader"))),
                2,
            ));
        }
    };
    if first.code != Some(0) {
        let mut o = not_pass(Reason::ReadFailed, ctx.clean.text(&first.stderr));
        if let Outcome::NotPass { refusal, .. } = &mut o {
            *refusal = true;
        }
        return Ok(at_step(o, 2));
    }
    let module = match Module::from_json(&first.stdout) {
        Ok(m) => m,
        Err(e) => {
            return Ok(at_step(
                not_pass(Reason::InvalidModule, ctx.clean.text(&e.to_string())),
                2,
            ));
        }
    };
    let again = match ctx.exec(p.reader, &arg, None)? {
        Ran::Done(e) => Some(e),
        Ran::TimedOut => None,
    };
    if !again.is_some_and(|s| s.code == Some(0) && s.stdout == first.stdout) {
        return Ok(at_step(
            not_pass(
                Reason::NotDeterministic,
                Some("read twice, with different exit status or bytes".into()),
            ),
            2,
        ));
    }
    let subject = l.case.subject();
    let used = features_used(&module)?;
    let refused_at_3 = |reason, detail: String| Outcome::NotPass {
        refusal: true,
        step: Some(3),
        reason,
        first_difference: None,
        detail: Some(detail),
    };
    if !used.contains(subject) {
        return Ok(refused_at_3(
            Reason::SubjectMissing,
            format!("the module does not use {subject} outside a foreign statement"),
        ));
    }
    if subject != "stmt.foreign" && used.contains("stmt.foreign") {
        return Ok(refused_at_3(
            Reason::ForeignStatement,
            format!(
                "the module keeps source as a foreign statement; {subject} counts only when \
                 nothing of its case is foreign"
            ),
        ));
    }
    let passed = codespec_passes::run(&module).to_json();
    let out = work.join("go");
    match write(ctx, p.go_writer, passed.as_bytes(), &out)?.0 {
        Wrote::Refused(why) => {
            return Ok(at_step(not_pass(Reason::Refused, ctx.clean.text(&why)), 4));
        }
        Wrote::TimedOut => {
            return Ok(at_step(
                not_pass(Reason::Timeout, Some(ctx.overran("the Go writer"))),
                4,
            ));
        }
        Wrote::Done => {}
    }
    let four = run_and_compare(
        ctx,
        p.go_runner,
        &work.join("go-build"),
        &out,
        calls,
        &l.expected,
    )?;
    Ok(at_step(four, 4))
}

/// Run a reader over its corpus, as the module doc says, each program within
/// [`DEFAULT_TIMEOUT`]. `runner` runs the reader's language; `go_writer` and `go_runner` are the
/// trusted Go writer and runner.
pub fn conform_reader(
    run: &Run,
    corpus: &Path,
    reader: &dyn Program,
    runner: &dyn Program,
    go_writer: &dyn Program,
    go_runner: &dyn Program,
) -> Result<Report, Error> {
    conform_reader_within(
        run,
        DEFAULT_TIMEOUT,
        corpus,
        reader,
        runner,
        go_writer,
        go_runner,
    )
}

/// [`conform_reader`] with each program limited to `limit`.
pub fn conform_reader_within(
    run: &Run,
    limit: Duration,
    corpus: &Path,
    reader: &dyn Program,
    runner: &dyn Program,
    go_writer: &dyn Program,
    go_runner: &dyn Program,
) -> Result<Report, Error> {
    let p = prepare(run, "reader", &[("corpus", corpus)])?;
    let corpus = corpus
        .canonicalize()
        .map_err(|e| format!("{}: {e}", corpus.display()))?;
    let ctx = Ctx {
        clean: Clean::new(&[
            (&p.work, "<work>"),
            (&run.kit, "<kit>"),
            (&corpus, "<corpus>"),
        ]),
        limit,
    };
    let mut names: Vec<String> = std::fs::read_dir(&corpus)
        .map_err(|e| format!("{}: {e}", corpus.display()))?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", corpus.display()))?;
    names.sort();
    let mut entries = Vec::new();
    for name in names {
        let case =
            p.cases.iter().find(|c| c.name == name).ok_or_else(|| {
                format!("{}/{name}: the kit has no case {name:?}", corpus.display())
            })?;
        let calls_path = corpus.join(&name).join("calls.json");
        read_calls(&calls_path)?;
        let mut l = load(case)?;
        l.calls = read_file(&calls_path)?;
        entries.push(l);
    }
    let corpus_ref = CorpusRef {
        sha256: tree_sha256(&corpus, &["."])?,
    };
    let calls_of = |l: &Loaded| p.work.join("cases").join(&l.case.name).join("calls.json");
    for l in &entries {
        put(&calls_of(l), &l.calls)?;
    }
    let programs = ReaderPrograms {
        reader,
        runner,
        go_writer,
        go_runner,
    };
    let ran = parallel(&entries, |l| {
        let o = reader_case(
            &ctx,
            l,
            &corpus.join(&l.case.name),
            &calls_of(l),
            &p.work.join("cases").join(&l.case.name),
            &programs,
        )?;
        Ok((l.case.name.clone(), l.case.subject().to_string(), o))
    })?;
    let features = verdicts(&p.kit, &p.claimed, &ran);
    Ok(finish(
        &p.manifest,
        run.manifest,
        p.kit_sha,
        Some(corpus_ref),
        features,
        Vec::new(),
    ))
}

/// Where and what to run for [`conform_adapter`].
#[derive(Debug, Clone)]
pub struct Target {
    /// The directory holding the programs of the module doc's first table.
    pub adapters: PathBuf,
    pub role: String,
    pub lang: String,
    pub kit: PathBuf,
    /// A reader's corpus; `None` is `<kit>/reader/<lang>`.
    pub corpus: Option<PathBuf>,
    pub work: PathBuf,
}

fn program(dir: &Path, name: &str) -> Result<Process, Error> {
    let p = std::path::absolute(dir.join(name)).map_err(|e| e.to_string())?;
    if p.is_file() {
        Ok(Process(p))
    } else {
        Err(format!("{} does not exist", p.display()))
    }
}

/// Run the kit against `codespec-<role>-<lang>` in `adapters`, by the manifest its `--describe`
/// prints, which must name the same role and language; each program within [`DEFAULT_TIMEOUT`].
pub fn conform_adapter(t: &Target) -> Result<Report, Error> {
    conform_adapter_within(t, DEFAULT_TIMEOUT)
}

/// [`conform_adapter`] with each program limited to `limit`.
pub fn conform_adapter_within(t: &Target, limit: Duration) -> Result<Report, Error> {
    let corpus = t
        .corpus
        .clone()
        .unwrap_or_else(|| t.kit.join("reader").join(&t.lang));
    let mut inputs = vec![("adapters", t.adapters.as_path()), ("kit", t.kit.as_path())];
    if t.role == "reader" {
        inputs.push(("corpus", corpus.as_path()));
    }
    check_work(&t.work, &inputs)?;
    let adapter = match t.role.as_str() {
        "reader" => program(&t.adapters, &format!("codespec-read-{}", t.lang))?,
        "writer" => program(&t.adapters, &format!("codespec-write-{}", t.lang))?,
        other => return Err(format!("role {other:?}, one of reader, writer")),
    };
    let manifest = adapter.describe()?;
    let m = codespec_model::manifest_from_json(&manifest)
        .map_err(|e| format!("{} --describe: {e}", adapter.0.display()))?;
    if m.lang != t.lang {
        return Err(format!(
            "{} --describe names language {:?}, not {:?}",
            adapter.0.display(),
            m.lang,
            t.lang
        ));
    }
    let runner = program(&t.adapters, &format!("codespec-run-{}", t.lang))?;
    let run = Run {
        manifest: &manifest,
        kit: t.kit.clone(),
        work: t.work.clone(),
    };
    if t.role == "writer" {
        return conform_writer_within(&run, limit, &adapter, &runner);
    }
    conform_reader_within(
        &run,
        limit,
        &corpus,
        &adapter,
        &runner,
        &program(&t.adapters, "codespec-write-go")?,
        &program(&t.adapters, "codespec-run-go")?,
    )
}

/// The languages whose reader corpus [`reader_corpus`] builds from the kit.
pub const CORPUS_LANGS: &[&str] = &["go", "rust", "java", "groovy", "c"];

/// The reader corpus of `lang`, one of [`CORPUS_LANGS`], built from the case sources of the kit
/// at `kit` as the module doc's table says: every file by its path relative to the corpus
/// directory (`<case>/…`, `/`-separated), in path order.
pub fn reader_corpus(kit: &Path, lang: &str) -> Result<Vec<(String, Vec<u8>)>, Error> {
    if !CORPUS_LANGS.contains(&lang) {
        return Err(format!(
            "no corpus is built for language {lang:?}, one of {CORPUS_LANGS:?}"
        ));
    }
    let mut out = Vec::new();
    for case in load_cases(&kit.join("cases"))? {
        let dir = &case.dir;
        let source = match lang {
            "go" => dir.join("case.go"),
            "rust" => dir.join("rust/case.rs"),
            "c" => dir.join("c/case.c"),
            "groovy" if dir.join("groovy/Case.groovy").is_file() => dir.join("groovy/Case.groovy"),
            _ => dir.join("java/Case.java"),
        };
        if !source.is_file() {
            continue;
        }
        let name = &case.name;
        let text = read_file(&source)?;
        match lang {
            "go" => out.push((format!("{name}/case.go"), text)),
            "rust" => {
                out.push((
                    format!("{name}/Cargo.toml"),
                    format!(
                        "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
                        crate::package_name(name)
                    )
                    .into_bytes(),
                ));
                out.push((format!("{name}/src/lib.rs"), text));
            }
            "java" => out.push((format!("{name}/Case.java"), text)),
            "c" => out.push((format!("{name}/case.c"), text)),
            _ => out.push((format!("{name}/Case.groovy"), text)),
        }
        out.push((
            format!("{name}/calls.json"),
            read_file(&dir.join("calls.json"))?,
        ));
    }
    out.sort();
    Ok(out)
}

/// One line on a report: the adapter, whether it conforms and the summary's counts.
pub fn summary_line(r: &Report) -> String {
    let s = &r.summary;
    format!(
        "{} {} {}: {}; {} claimed, {} pass, {} fail, {} refused unclaimed, {} pass unclaimed, {} untested, {} boundary failing",
        r.adapter.role,
        r.adapter.lang,
        r.adapter.version,
        if r.conforms {
            "conforms"
        } else {
            "does not conform"
        },
        s.claimed,
        s.pass,
        s.fail,
        s.refused_unclaimed,
        s.pass_unclaimed,
        s.untested,
        s.boundary_fail
    )
}

/// What differs between a report and a fresh one, one line each: every feature whose entry
/// changed, by id, then the adapter, kit, corpus, boundary cases and `conforms`.
pub fn differences(old: &Report, new: &Report) -> Vec<String> {
    let mut out = Vec::new();
    let line = |f: &FeatureResult| {
        let mut s = f.verdict.to_string();
        if let Some(c) = &f.case {
            s += &format!(" ({c}");
            if let Some(r) = f.reason {
                s += &format!(", {r}");
            }
            if let Some(d) = &f.first_difference {
                s += &format!(
                    ", line {}: expected {:?}, got {:?}",
                    d.line, d.expected, d.got
                );
            }
            s += ")";
        }
        s
    };
    for n in &new.features {
        match old.features.iter().find(|o| o.id == n.id) {
            None => out.push(format!("{}: not in the report, now {}", n.id, line(n))),
            Some(o) if o != n => out.push(format!("{}: was {}, now {}", n.id, line(o), line(n))),
            Some(_) => {}
        }
    }
    for o in &old.features {
        if !new.features.iter().any(|n| n.id == o.id) {
            out.push(format!("{}: in the report, no longer in the kit", o.id));
        }
    }
    if old.adapter != new.adapter {
        out.push("adapter: its manifest or version changed".into());
    }
    if old.kit != new.kit {
        out.push("kit: its digest or version changed".into());
    }
    if old.corpus != new.corpus {
        out.push("corpus: its digest changed".into());
    }
    if old.boundary != new.boundary {
        out.push("boundary: a boundary case changed".into());
    }
    if old.codespec != new.codespec {
        out.push(format!(
            "codespec: was {}, now {}",
            old.codespec, new.codespec
        ));
    }
    if out.is_empty() && old != new {
        out.push("the report differs".into());
    }
    out
}
