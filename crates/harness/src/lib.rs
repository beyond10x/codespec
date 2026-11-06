//! The comparison harness: runs every implementation of one example on seeded traces and requires
//! their outputs to be byte-equal.
//!
//! # Manifest
//!
//! `<example>/harness.toml` lists the implementations, the first being the reference:
//!
//! ```toml
//! [[implementation]]
//! name = "original"
//! build = ["go", "build", "-C", "drivers/go", "-o", "{build}/original", "."]
//! run = ["{build}/original"]
//! ```
//!
//! `build` and `run` are argument vectors, not shell lines, run with the example directory as the
//! working directory. `{build}` in any argument becomes the absolute build directory (`--build-dir`,
//! default `target/harness`). Unknown fields, an empty `run`, an empty list and duplicate names are
//! refused.
//!
//! # Trace
//!
//! A trace is UTF-8 text, one command per line, each line ending in `\n`, fields separated by one
//! space; no field is empty or holds a space. The first command is `new`.
//!
//! | line | call |
//! |---|---|
//! | `new` | a fresh service on a fresh event channel, replacing the previous one |
//! | `place <customer> <total> <gift>` | `Place(customer, total, gift)`; `total` a decimal `int64`, `gift` `true` or `false` |
//! | `pay <id>` | `Pay(id)` |
//! | `ship <id> <tracking>` | `Ship(id, tracking)` |
//! | `cancel <id>` | `Cancel(id)` |
//! | `get <id>` | `Get(id)` |
//! | `pending` | `PendingOrders()` |
//!
//! There is no command for `Audit`: the Rust output has no such method (the writer refuses its
//! goroutine), so a trace could not be run against every implementation.
//!
//! The generator is [`SplitMix64`] seeded with the seed, and only integer arithmetic on its outputs
//! decides a trace, so one seed gives the same bytes on every platform. About a fifth of totals are
//! zero, negative, one or an `int64` extreme, and about a fifth of ids are ones the service never
//! issued (`o-0`, ids past the last, `x`, `O-1`, `o-01`), so every error path is reached.
//!
//! # Output
//!
//! A driver prints one line per command, then one line per event the call sent, drained from the
//! channel in order. `<order>` is `id=<id> customer=<c> total=<t> status=<s> tracking=<t>
//! gift=<true|false>`; errors are their `Error()` (Go) or `Display` (Rust) text.
//!
//! | command | lines |
//! |---|---|
//! | `new` | `new` |
//! | `place` | `place ok <id>` or `place err <error>` |
//! | `pay`, `ship`, `cancel` | `<cmd> ok` or `<cmd> err <error>` |
//! | `get` | `get found <order>` or `get missing` |
//! | `pending` | `pending <n>` followed by ` \| <order>` per order, sorted by id, since Go map order is random |
//! | each event | `event OrderPlaced id=<id> customer=<c> total=<t>`, `event OrderPaid id=<id>`, `event OrderShipped id=<id> tracking=<t>`, `event OrderCancelled id=<id>` |
//!
//! # Comparison
//!
//! For each seed `0..seeds`, every driver gets the trace on stdin; a driver that exits non-zero
//! fails the run. Every output is compared byte for byte with the reference's; the first that
//! differs fails the run with the seed, the implementation and the first differing line.

use std::collections::BTreeSet;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

/// The splitmix64 generator (Steele, Lea and Flood): a 64-bit state advanced by a fixed odd
/// constant and mixed by two multiply-xorshift rounds.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A value in `0..n`, `n > 0`, by remainder; the bias is irrelevant here and fixed.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len() as u64) as usize]
    }
}

const CUSTOMERS: &[&str] = &["alice", "bob", "carol", "dave", "zoë", "o-1", "Ω"];
const BAD_IDS: &[&str] = &["o-0", "x", "O-1", "o-01", "o--1"];
const TRACKING: &[&str] = &["TRK", "1Z999", "t", "Ω-7"];
const EDGE_TOTALS: &[i64] = &[0, -1, 1, i64::MAX, i64::MIN, i64::MIN + 1, -1000];

/// The trace of `commands` commands for `seed`.
pub fn trace(seed: u64, commands: usize) -> String {
    let mut r = SplitMix64(seed);
    let mut out = String::new();
    let mut placed: u64 = 0;
    for i in 0..commands {
        let roll = r.below(100);
        if i == 0 || roll < 2 {
            placed = 0;
            out.push_str("new\n");
            continue;
        }
        let line = match roll {
            2..=25 => {
                let customer = r.pick(CUSTOMERS);
                let total = if r.below(5) == 0 {
                    EDGE_TOTALS[r.below(EDGE_TOTALS.len() as u64) as usize]
                } else {
                    1 + r.below(1000) as i64
                };
                if total > 0 {
                    placed += 1;
                }
                format!("place {customer} {total} {}", r.below(2) == 1)
            }
            26..=40 => format!("pay {}", id(&mut r, placed)),
            41..=55 => {
                let id = id(&mut r, placed);
                let tracking = if r.below(2) == 0 {
                    r.pick(TRACKING).to_string()
                } else {
                    format!("t-{}", r.below(10_000))
                };
                format!("ship {id} {tracking}")
            }
            56..=67 => format!("cancel {}", id(&mut r, placed)),
            68..=85 => format!("get {}", id(&mut r, placed)),
            _ => "pending".to_string(),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn id(r: &mut SplitMix64, placed: u64) -> String {
    match r.below(10) {
        0..=7 if placed > 0 => format!("o-{}", 1 + r.below(placed)),
        0..=8 => format!("o-{}", placed + 1 + r.below(3)),
        _ => r.pick(BAD_IDS).to_string(),
    }
}

/// `harness.toml`: the implementations of one example, the first the reference.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub implementation: Vec<Implementation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Implementation {
    pub name: String,
    pub build: Vec<String>,
    pub run: Vec<String>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let m: Manifest = toml::from_str(text).map_err(|e| e.to_string())?;
        if m.implementation.is_empty() {
            return Err("no implementation".into());
        }
        let mut names = BTreeSet::new();
        for i in &m.implementation {
            if i.name.is_empty() || !names.insert(i.name.as_str()) {
                return Err(format!(
                    "implementation name {:?} empty or repeated",
                    i.name
                ));
            }
            if i.run.is_empty() {
                return Err(format!("implementation {:?}: empty run", i.name));
            }
        }
        Ok(m)
    }

    pub fn load(example: &Path) -> Result<Manifest, String> {
        let path = example.join("harness.toml");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Manifest::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

pub struct Options {
    pub example: PathBuf,
    pub build_dir: PathBuf,
    pub seeds: u64,
    pub commands: usize,
}

/// A built implementation: its name, the program to run and the directory to run it in.
pub struct Built {
    pub name: String,
    run: Vec<String>,
    dir: PathBuf,
}

fn absolute(p: &Path) -> Result<PathBuf, String> {
    std::path::absolute(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn substitute(args: &[String], build: &Path) -> Vec<String> {
    let b = build.to_string_lossy();
    args.iter().map(|a| a.replace("{build}", &b)).collect()
}

/// Build every implementation the manifest lists, in order.
pub fn build(opts: &Options) -> Result<Vec<Built>, Failure> {
    let example = absolute(&opts.example).map_err(Failure::Setup)?;
    let m = Manifest::load(&example).map_err(Failure::Setup)?;
    let build_dir = absolute(&opts.build_dir).map_err(Failure::Setup)?;
    std::fs::create_dir_all(&build_dir)
        .map_err(|e| Failure::Setup(format!("{}: {e}", build_dir.display())))?;
    let mut built = Vec::new();
    for i in m.implementation {
        let args = substitute(&i.build, &build_dir);
        if let Some((prog, rest)) = args.split_first() {
            let out = Command::new(prog)
                .args(rest)
                .current_dir(&example)
                .stdin(Stdio::null())
                .output()
                .map_err(|e| Failure::Setup(format!("{}: build: {prog}: {e}", i.name)))?;
            if !out.status.success() {
                return Err(Failure::Setup(format!(
                    "{}: build {:?} exited with {}\n{}{}",
                    i.name,
                    args,
                    out.status,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                )));
            }
        }
        built.push(Built {
            run: substitute(&i.run, &build_dir),
            name: i.name,
            dir: example.clone(),
        });
    }
    Ok(built)
}

/// Run one built driver on a trace and return its stdout; a non-zero exit is an error.
pub fn run_driver(built: &Built, trace: &str) -> Result<Vec<u8>, String> {
    let (prog, rest) = built.run.split_first().expect("run is non-empty");
    let mut child = Command::new(prog)
        .args(rest)
        .current_dir(&built.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{prog}: {e}"))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = trace.as_bytes().to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("{prog}: {e}"))?;
    let written = writer.join().expect("stdin writer");
    if !out.status.success() {
        return Err(format!(
            "exited with {}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    written.map_err(|e| format!("writing the trace: {e}"))?;
    Ok(out.stdout)
}

/// Build every implementation and compare them on seeds `0..opts.seeds`.
pub fn check(opts: &Options) -> Result<Summary, Failure> {
    let built = build(opts)?;
    let mut lines = 0u64;
    for seed in 0..opts.seeds {
        let t = trace(seed, opts.commands);
        let mut reference: Option<Vec<u8>> = None;
        for b in &built {
            let out = run_driver(b, &t).map_err(|message| Failure::Driver {
                seed,
                name: b.name.clone(),
                message,
            })?;
            match &reference {
                None => {
                    lines += out.iter().filter(|&&c| c == b'\n').count() as u64;
                    reference = Some(out);
                }
                Some(want) if *want != out => {
                    let (line, expected, got) = first_difference(want, &out);
                    return Err(Failure::Mismatch {
                        seed,
                        reference: built[0].name.clone(),
                        name: b.name.clone(),
                        line,
                        expected,
                        got,
                    });
                }
                Some(_) => {}
            }
        }
    }
    Ok(Summary {
        seeds: opts.seeds,
        commands: opts.commands,
        implementations: built.into_iter().map(|b| b.name).collect(),
        lines,
    })
}

/// The 1-based number of the first line, newline included, on which `a` and `b` differ, and that
/// line of each (`None` past the end).
fn first_difference(a: &[u8], b: &[u8]) -> (usize, Option<String>, Option<String>) {
    let mut la = a.split_inclusive(|&c| c == b'\n');
    let mut lb = b.split_inclusive(|&c| c == b'\n');
    let mut n = 0;
    loop {
        n += 1;
        let (x, y) = (la.next(), lb.next());
        if x != y {
            let text = |l: Option<&[u8]>| l.map(|l| String::from_utf8_lossy(l).into_owned());
            return (n, text(x), text(y));
        }
        if x.is_none() {
            return (n, None, None);
        }
    }
}

#[derive(Debug)]
pub struct Summary {
    pub seeds: u64,
    pub commands: usize,
    pub implementations: Vec<String>,
    /// Output lines of the reference over every seed; every other implementation printed the same.
    pub lines: u64,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "harness: {} seeds x {} commands, {} implementations ({}), {} output lines each, byte-equal",
            self.seeds,
            self.commands,
            self.implementations.len(),
            self.implementations.join(", "),
            self.lines
        )
    }
}

#[derive(Debug)]
pub enum Failure {
    Setup(String),
    Driver {
        seed: u64,
        name: String,
        message: String,
    },
    Mismatch {
        seed: u64,
        reference: String,
        name: String,
        line: usize,
        expected: Option<String>,
        got: Option<String>,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = |l: &Option<String>| match l {
            Some(l) => format!("{:?}", l.strip_suffix('\n').unwrap_or(l)),
            None => "<end of output>".to_string(),
        };
        match self {
            Failure::Setup(m) => f.write_str(m),
            Failure::Driver {
                seed,
                name,
                message,
            } => write!(f, "seed {seed}: {name}: {message}"),
            Failure::Mismatch {
                seed,
                reference,
                name,
                line: n,
                expected,
                got,
            } => {
                let note = match (expected, got) {
                    (Some(e), Some(g)) if e.trim_end_matches('\n') == g.trim_end_matches('\n') => {
                        " (line endings differ)"
                    }
                    _ => "",
                };
                write!(
                    f,
                    "seed {seed}: {name} differs from {reference} at line {n}{note}\n  expected: {}\n  got:      {}",
                    line(expected),
                    line(got)
                )
            }
        }
    }
}
