//! The candidate snapshot `codespec-corpus-candidates/1` and `fetch`, which writes it from `gh`.

use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;

/// The `format` of a snapshot.
pub const FORMAT: &str = "codespec-corpus-candidates/1";

/// The languages `codespec read --lang` has an adapter for and the corpus has a driver for: the
/// name GitHub gives the language, and the `--lang` id of the reader.
pub const LANGUAGES: &[(&str, &str)] = &[("Go", "go"), ("Rust", "rust"), ("Java", "java")];

/// The GitHub name and the reader id of `language` (any case), or a refusal that names it.
pub fn language(language: &str) -> Result<(&'static str, &'static str), Error> {
    LANGUAGES
        .iter()
        .find(|(github, _)| github.eq_ignore_ascii_case(language))
        .copied()
        .ok_or_else(|| {
            let known: Vec<&str> = LANGUAGES.iter().map(|(_, reader)| *reader).collect();
            Error(format!(
                "language: {language}: `codespec read` has no adapter for it (the readers the corpus drives: {})",
                known.join(", ")
            ))
        })
}

/// Repositories left out, counted among the rows `fetch` looked at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Excluded {
    pub forks: u64,
    pub archived: u64,
    pub oversized: u64,
}

/// One repository of a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    /// `owner/repo`.
    pub name: String,
    pub url: String,
    /// The language GitHub gives the repository; equal to the snapshot's.
    pub language: String,
    /// The default branch's commit when the snapshot was fetched: 40 lowercase hex.
    pub commit: String,
    pub stars: u64,
    pub size_kb: u64,
    /// 1 for the most starred.
    pub rank: u64,
}

/// A candidate snapshot, read strictly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub format: String,
    /// UTC date of the query, `YYYY-MM-DD`.
    pub fetched_on: String,
    /// The GitHub name of the language (`Go`).
    pub language: String,
    pub max_size_kb: u64,
    pub excluded: Excluded,
    pub candidates: Vec<Candidate>,
}

fn is_commit(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        text.get(range)
            .filter(|t| t.bytes().all(|c| c.is_ascii_digit()))
            .and_then(|t| t.parse().ok())
    };
    matches!(
        (number(0..4), number(5..7), number(8..10)),
        (Some(_), Some(1..=12), Some(1..=31))
    )
}

fn is_repository_name(text: &str) -> bool {
    let mut parts = text.split('/');
    let (owner, repo) = (parts.next(), parts.next());
    parts.next().is_none()
        && [owner, repo].iter().all(|part| {
            part.is_some_and(|p| {
                !p.is_empty()
                    && p.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            })
        })
}

impl Snapshot {
    /// Reads a snapshot and checks every rule of the crate doc, in the order of the fields.
    pub fn parse(bytes: &[u8]) -> Result<Snapshot, Error> {
        let snapshot: Snapshot =
            serde_json::from_slice(bytes).map_err(|e| Error(format!("snapshot: {e}")))?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.format != FORMAT {
            return Err(Error(format!(
                "format: {:?} is not {FORMAT:?}",
                self.format
            )));
        }
        if !is_date(&self.fetched_on) {
            return Err(Error(format!(
                "fetched_on: {:?} is not a date YYYY-MM-DD",
                self.fetched_on
            )));
        }
        let (github, _) = language(&self.language)?;
        if self.language != github {
            return Err(Error(format!(
                "language: {:?} is not spelled {github:?}",
                self.language
            )));
        }
        let mut names = std::collections::BTreeSet::new();
        let mut ranks = std::collections::BTreeSet::new();
        for (i, c) in self.candidates.iter().enumerate() {
            if !is_repository_name(&c.name) {
                return Err(Error(format!(
                    "candidates[{i}].name: {:?} is not owner/repo",
                    c.name
                )));
            }
            if !names.insert(c.name.as_str()) {
                return Err(Error(format!(
                    "candidates[{i}].name: {:?} is listed twice",
                    c.name
                )));
            }
            if !c.url.starts_with("https://") {
                return Err(Error(format!(
                    "candidates[{i}].url: {:?} of {} does not start with https://",
                    c.url, c.name
                )));
            }
            if c.language != self.language {
                return Err(Error(format!(
                    "candidates[{i}].language: {:?} of {} differs from the snapshot's {:?}",
                    c.language, c.name, self.language
                )));
            }
            if !is_commit(&c.commit) {
                return Err(Error(format!(
                    "candidates[{i}].commit: {:?} of {} is not 40 lowercase hex",
                    c.commit, c.name
                )));
            }
            if c.rank == 0 {
                return Err(Error(format!(
                    "candidates[{i}].rank: 0 of {} is below 1",
                    c.name
                )));
            }
            if !ranks.insert(c.rank) {
                return Err(Error(format!(
                    "candidates[{i}].rank: {} of {} is used twice",
                    c.rank, c.name
                )));
            }
        }
        Ok(())
    }

    /// The candidates in rank order.
    pub fn by_rank(&self) -> Vec<&Candidate> {
        let mut rows: Vec<&Candidate> = self.candidates.iter().collect();
        rows.sort_by_key(|c| c.rank);
        rows
    }

    /// The bytes `fetch` writes: the snapshot in rank order, two-space indent, a final newline.
    pub fn canonical(&self) -> String {
        let mut ordered = self.clone();
        ordered.candidates.sort_by_key(|c| c.rank);
        let mut text = serde_json::to_string_pretty(&ordered).expect("a snapshot is JSON");
        text.push('\n');
        text
    }

    /// The digest a manifest records: sha256 of [`Snapshot::canonical`], so the order of the
    /// rows in the file does not change it.
    pub fn digest(&self) -> String {
        Sha256::digest(self.canonical().as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// What `fetch` is asked.
#[derive(Debug)]
pub struct Fetch {
    pub language: String,
    pub count: usize,
    pub max_size_kb: u64,
    /// `YYYY-MM-DD`; today's UTC date when not given.
    pub fetched_on: Option<String>,
    /// The program run as `gh`.
    pub gh: String,
}

/// The fields of one row of `gh search repos --json`, the others ignored.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    full_name: String,
    url: String,
    language: Option<String>,
    stargazers_count: u64,
    size: u64,
    is_fork: bool,
    is_archived: bool,
    default_branch: String,
}

fn gh(program: &str, args: &[&str]) -> Result<Vec<u8>, Error> {
    let shown = args.iter().take(2).copied().collect::<Vec<_>>().join(" ");
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| Error(format!("--gh: cannot run {program}: {e}")))?;
    if !out.status.success() {
        return Err(Error(format!(
            "--gh: {program} {shown} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(out.stdout)
}

/// The snapshot of the top `count` usable repositories of one language by stars.
///
/// Asks `gh` for the top `min(1000, 2 * count + 10)` repositories of the language by stars, forks
/// included, and walks them in that order until `count` are usable; a fork, an archived
/// repository and one over `max_size_kb` are left out and counted, a row that is two of these
/// counts as the first of fork, archived, oversized. For each usable one `gh api` gives the commit
/// of its default branch. Fewer than `count` usable rows is a refusal.
pub fn fetch(options: &Fetch) -> Result<Snapshot, Error> {
    let (github, _) = language(&options.language).map_err(|e| Error(format!("--{e}")))?;
    if options.count == 0 {
        return Err(Error("--count: 0 is below 1".to_owned()));
    }
    let fetched_on = match &options.fetched_on {
        Some(date) if is_date(date) => date.clone(),
        Some(date) => {
            return Err(Error(format!(
                "--fetched-on: {date:?} is not a date YYYY-MM-DD"
            )));
        }
        None => today(),
    };
    let limit = (2 * options.count + 10).min(1000).to_string();
    let found = gh(
        &options.gh,
        &[
            "search",
            "repos",
            "--language",
            github,
            "--sort",
            "stars",
            "--order",
            "desc",
            "--include-forks=true",
            "--limit",
            &limit,
            "--json",
            "fullName,url,language,stargazersCount,size,isFork,isArchived,defaultBranch",
        ],
    )?;
    let rows: Vec<Row> = serde_json::from_slice(&found).map_err(|e| {
        Error(format!(
            "--gh: the search output is not the expected JSON: {e}"
        ))
    })?;
    let mut excluded = Excluded {
        forks: 0,
        archived: 0,
        oversized: 0,
    };
    let mut candidates = Vec::new();
    for row in &rows {
        if candidates.len() == options.count {
            break;
        }
        if row.is_fork {
            excluded.forks += 1;
        } else if row.is_archived {
            excluded.archived += 1;
        } else if row.size > options.max_size_kb {
            excluded.oversized += 1;
        } else {
            let sha = gh(
                &options.gh,
                &[
                    "api",
                    &format!("repos/{}/commits/{}", row.full_name, row.default_branch),
                ],
            )?;
            let commit = serde_json::from_slice::<serde_json::Value>(&sha)
                .ok()
                .and_then(|v| v["sha"].as_str().map(str::to_owned))
                .ok_or_else(|| {
                    Error(format!(
                        "--gh: the commit of {} is not JSON with a sha",
                        row.full_name
                    ))
                })?;
            candidates.push(Candidate {
                name: row.full_name.clone(),
                url: row.url.clone(),
                language: row.language.clone().unwrap_or_default(),
                commit,
                stars: row.stargazers_count,
                size_kb: row.size,
                rank: candidates.len() as u64 + 1,
            });
        }
    }
    if candidates.len() < options.count {
        return Err(Error(format!(
            "--count: {} is more than the {} usable {github} repositories among the {} rows gh returned ({} forks, {} archived, {} over {} KB left out)",
            options.count,
            candidates.len(),
            rows.len(),
            excluded.forks,
            excluded.archived,
            excluded.oversized,
            options.max_size_kb
        )));
    }
    let snapshot = Snapshot {
        format: FORMAT.to_owned(),
        fetched_on,
        language: github.to_owned(),
        max_size_kb: options.max_size_kb,
        excluded,
        candidates,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

/// Today's UTC date.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil(secs / 86_400);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The proleptic Gregorian date of `days` since 1970-01-01.
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + u64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::{civil, is_commit, is_date, is_repository_name};

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(20_417), (2025, 11, 25));
        assert_eq!(civil(19_782), (2024, 2, 29));
    }

    #[test]
    fn shapes() {
        assert!(is_commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_commit("0123456789ABCDEF0123456789abcdef01234567"));
        assert!(is_date("2026-11-25") && !is_date("2026-13-01") && !is_date("26-11-2026"));
        assert!(is_repository_name("a/b.c-d_e") && !is_repository_name("a/b/c"));
        assert!(
            !is_repository_name("a") && !is_repository_name("/b") && !is_repository_name("a/b c")
        );
    }
}
