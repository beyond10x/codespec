//! `codespec-corpus`: the repositories a reader is measured on, chosen so that the same inputs give
//! the same list.
//!
//! Star rankings move daily, so the determinism comes from a snapshot, not from the query. Two
//! steps:
//!
//! 1. `codespec-corpus fetch --language <name> --count <K> --out <candidates.json>` asks `gh` and
//!    writes a candidate snapshot;
//! 2. `codespec-corpus select --candidates <candidates.json> --mode top|random --count <N>
//!    [--seed <S>] --out <manifest.json>` writes the manifest, a pure function of the snapshot and
//!    its arguments. `select --check <manifest.json> --candidates <candidates.json>` derives the
//!    manifest again from the mode, count and seed it records and compares.
//!
//! No source of any repository is copied: both files hold names, URLs and commits, and a few
//! counts. Reading, triage and the rest of the loop are other stories of
//! `epic:reader-self-improvement-loop`; each adds a subcommand here and a module to
//! `tests/main.rs`.
//!
//! # Languages
//!
//! The languages `codespec read --lang` has an adapter for and the corpus drives, by the name
//! GitHub gives them and the id of the reader: `Go` `go`, `Rust` `rust`, `Java` `java`
//! ([`snapshot::LANGUAGES`]). `--language` takes either spelling in any case; both files hold the
//! GitHub name.
//!
//! # `codespec-corpus-candidates/1`, the snapshot
//!
//! One JSON object, read strictly (an unknown field is refused), written with a two-space indent
//! and a final newline, candidates in rank order:
//!
//! | field | what |
//! |---|---|
//! | `format` | `codespec-corpus-candidates/1` |
//! | `fetched_on` | the UTC date of the query, `YYYY-MM-DD` |
//! | `language` | the GitHub name of the language |
//! | `max_size_kb` | the `--max-size-kb` of the fetch (default 200000): a larger repository was left out |
//! | `excluded` | `{forks, archived, oversized}`: the rows left out, counted among the rows the fetch looked at |
//! | `candidates` | the repositories, each `{name, url, language, commit, stars, size_kb, rank}` |
//!
//! A candidate's `name` is `owner/repo`, its `url` the repository's `https://` URL, its `language`
//! the language GitHub gives it (equal to the snapshot's), its `commit` the default branch's commit
//! at `fetched_on` (40 lowercase hex), `stars` and `size_kb` as GitHub reported them, `rank` 1 for
//! the most starred.
//!
//! `fetch` runs `gh search repos --language <Name> --sort stars --order desc --include-forks=true
//! --limit <min(1000, 2K + 10)> --json ...`, walks the rows in that order until K are usable and
//! asks `gh api repos/<name>/commits/<default branch>` for each usable one. A fork, an archived
//! repository and one over `--max-size-kb` are counted in `excluded` and not listed (a row that
//! is two of these counts as the first of fork, archived, oversized). Ranks are 1 to K in the
//! order `gh` gave, after the exclusions.
//!
//! # `codespec-corpus/1`, the manifest
//!
//! One JSON object, read strictly, two-space indent, final newline:
//!
//! | field | what |
//! |---|---|
//! | `format` | `codespec-corpus/1` |
//! | `language` | the snapshot's `language` |
//! | `reader` | the `codespec read --lang` id |
//! | `mode` | `top` or `random` |
//! | `count` | N |
//! | `seed` | the seed of `random`, `null` for `top` |
//! | `fetched_on` | the snapshot's |
//! | `snapshot_sha256` | the digest of the snapshot, below |
//! | `repositories` | N of `{name, url, commit}`, in rank order |
//!
//! The digest is the sha256 of the snapshot re-written canonically (candidates in rank order, the
//! layout above), so the order of the rows in the file and its whitespace do not change it; for a
//! file `fetch` wrote it is the sha256 of the file.
//!
//! # The selection
//!
//! `top` takes the N candidates of the lowest rank. `random` takes N by a shuffle fixed here, so the
//! same snapshot, N and seed give the same bytes on any host, whatever the order of the rows in
//! the file:
//!
//! 1. sort the candidates by rank; `a` is the list of their indices `0..n`;
//! 2. splitmix64 from the seed (state `s = seed`; each draw: `s = s + 0x9E3779B97F4A7C15` mod
//!    2^64, `z = s`, `z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9` mod 2^64, `z = (z ^ (z >> 27)) *
//!    0x94D049BB133111EB` mod 2^64, the draw is `z ^ (z >> 31)`; the first draw of seed 0 is
//!    `0xE220A8397B1DCDAF`);
//! 3. Fisher-Yates, for `i` from `n - 1` down to 1: draw `r`, `j = r mod (i + 1)`, swap `a[i]` and
//!    `a[j]` (the modulo bias is below `n / 2^64`, and fixed);
//! 4. the chosen are `a[0..N]`, listed in rank order.
//!
//! # Refusals
//!
//! A refusal is one line on stderr, `codespec-corpus: <field>: <what is wrong>`, exit 1, and
//! nothing is written. The field is a flag (`--count`) or a path in the file
//! (`candidates[3].commit`).
//!
//! | refused | field |
//! |---|---|
//! | a language `codespec read` has no adapter for (`Cobol`), in `fetch --language` or in a snapshot | `--language`, `language` |
//! | `--gh` that cannot be run, or that fails | `--gh` |
//! | a `--fetched-on` that is not a date | `--fetched-on` |
//! | `--count` 0; above the candidates of the snapshot; above the usable repositories `fetch` found | `--count` |
//! | `--mode` or `--count` missing from a `select` without `--check`, `--out` missing | `--mode`, `--count`, `--out` |
//! | `--mode random` without `--seed` | `--seed` |
//! | `--seed` with `--mode top` | `--seed` |
//! | a snapshot or manifest that is not JSON of the format above, or has an unknown field | the field |
//! | a snapshot whose `format` is not its format, `fetched_on` not a date, `language` not the GitHub spelling | `format`, `fetched_on`, `language` |
//! | a candidate whose `name` is not `owner/repo` or is listed twice | `candidates[i].name` |
//! | a candidate whose `url` does not start with `https://` | `candidates[i].url` |
//! | a candidate whose `language` differs from the snapshot's | `candidates[i].language` |
//! | a candidate whose `commit` is not 40 lowercase hex (also in `fetch`, from `gh`) | `candidates[i].commit` |
//! | a candidate whose `rank` is 0 or used twice | `candidates[i].rank` |
//! | `select --check` of a manifest of another `format` | `format` |
//! | `select --check` of a manifest whose recorded snapshot digest differs from the file it names | `snapshot_sha256` |
//! | `select --check` of a manifest that differs from what its mode, count and seed derive | the first differing field, in the order of the table above |
//!
//! # Tests
//!
//! `tests/main.rs` runs the binary over a stub `gh` (POSIX sh, `tests/fixtures/gh`) that prints
//! recorded JSON: no network, no model call. `candidates.expected.json` is the snapshot `fetch`
//! writes over it; `top3`, `random-7-3` and `random-8-3` `.expected.json` are the manifests of
//! that snapshot.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

pub mod select;
pub mod snapshot;

use select::Mode;
use snapshot::{Fetch, Snapshot};

/// A refusal: one line, which names the field.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// The command line of `codespec-corpus`.
#[derive(Debug, Parser)]
#[command(name = "codespec-corpus", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write a candidate snapshot from `gh`: the top repositories of one language by stars.
    Fetch {
        /// The language: go, rust or java (any case).
        #[arg(long)]
        language: String,
        /// How many usable repositories the snapshot holds.
        #[arg(long)]
        count: usize,
        /// The snapshot file to write.
        #[arg(long)]
        out: PathBuf,
        /// Repositories larger than this (GitHub's `size`, in KB) are left out and counted.
        #[arg(long, default_value_t = 200_000)]
        max_size_kb: u64,
        /// The date to record, `YYYY-MM-DD`; today's UTC date when not given.
        #[arg(long)]
        fetched_on: Option<String>,
        /// The GitHub CLI program.
        #[arg(long, default_value = "gh")]
        gh: String,
    },
    /// Write a manifest from a snapshot, or with --check re-derive one and compare.
    Select {
        /// The candidate snapshot.
        #[arg(long)]
        candidates: PathBuf,
        /// top: the first N by rank; random: N by the seeded shuffle.
        #[arg(long, value_enum, conflicts_with = "check")]
        mode: Option<Mode>,
        /// N.
        #[arg(long, conflicts_with = "check")]
        count: Option<usize>,
        /// The seed: required with random, refused with top.
        #[arg(long, conflicts_with = "check")]
        seed: Option<u64>,
        /// The manifest file to write.
        #[arg(long, conflicts_with = "check")]
        out: Option<PathBuf>,
        /// Check this manifest against --candidates instead of writing one.
        #[arg(long)]
        check: Option<PathBuf>,
    },
}

fn read(flag: &str, path: &Path) -> Result<Vec<u8>, Error> {
    std::fs::read(path).map_err(|e| Error(format!("{flag}: cannot read {}: {e}", path.display())))
}

fn write(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| Error(format!("--out: cannot create {}: {e}", dir.display())))?;
    }
    std::fs::write(path, text)
        .map_err(|e| Error(format!("--out: cannot write {}: {e}", path.display())))
}

/// Runs one command, printing one line on stdout when it succeeds.
pub fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Fetch {
            language,
            count,
            out,
            max_size_kb,
            fetched_on,
            gh,
        } => {
            let snapshot = snapshot::fetch(&Fetch {
                language,
                count,
                max_size_kb,
                fetched_on,
                gh,
            })?;
            write(&out, &snapshot.canonical())?;
            println!(
                "wrote {}: {} {} candidates of {} ({} forks, {} archived, {} oversized left out)",
                out.display(),
                snapshot.candidates.len(),
                snapshot.language,
                snapshot.fetched_on,
                snapshot.excluded.forks,
                snapshot.excluded.archived,
                snapshot.excluded.oversized
            );
            Ok(())
        }
        Command::Select {
            candidates,
            mode,
            count,
            seed,
            out,
            check,
        } => {
            let snapshot = Snapshot::parse(&read("--candidates", &candidates)?)?;
            if let Some(manifest) = check {
                select::check(&read("--check", &manifest)?, &snapshot)?;
                println!(
                    "{} is what select derives from {}",
                    manifest.display(),
                    candidates.display()
                );
                return Ok(());
            }
            let manifest = select::derive(&snapshot, mode, count, seed)?;
            let out = out.ok_or_else(|| Error("--out: required".to_owned()))?;
            write(&out, &manifest.canonical())?;
            println!(
                "wrote {}: {} {} repositories",
                out.display(),
                manifest.repositories.len(),
                manifest.language
            );
            Ok(())
        }
    }
}
