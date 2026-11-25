---
format: aep.planning-md/3
id: story:corpus-manifest
kind: story
status: draft
title: A corpus manifest of N public repositories, pinned to commits, same inputs same list
summary: 'codespec-corpus fetch and select: a candidate snapshot from gh and a seeded top-N or random manifest of name, url, commit.'
relations:
- decomposes: epic:reader-self-improvement-loop
- informed_by: story:java-reader-public-corpus
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: corpus
- confidence: inferred
  path: crates/corpus
- confidence: inferred
  path: scripts/check-parts.txt
revision: 2
---
# Story: A corpus manifest of N public repositories, pinned to commits, same inputs same list

## Outcome

A new crate `crates/corpus` (`b10x-codespec-corpus`, binary `codespec-corpus`) selects the
repositories a run reads, in two steps:

- `codespec-corpus fetch --language <name> --count <K> --out <candidates.json>` asks `gh` for the
  top K public repositories of one GitHub language by stars (`gh search repos --language <l>
  --sort stars`) and for the default-branch commit of each, and writes the candidate snapshot
  `codespec-corpus-candidates/1`: `fetched_on` (UTC date), `language`, and per candidate `name`
  (`owner/repo`), `url`, `commit` (40 hex), `stars`, `size_kb`, `rank`. Forks, archived
  repositories and repositories over `--max-size-kb` are left out and counted in the file.
- `codespec-corpus select --candidates <file> --mode top|random --count <N> [--seed <S>]
  --out <manifest.json>` writes the manifest `codespec-corpus/1`: `language`, `reader` (the
  `codespec read --lang` id: `go`, `rust`, `java`), `mode`, `count`, `seed`, the `fetched_on` and
  sha256 digest of the snapshot it came from, and per repository `name`, `url`, `commit`. No
  source is copied; the file holds nothing else about a repository.

`select` is a pure function of the snapshot and its arguments: `top` takes the first N by rank;
`random` takes N by a seeded shuffle whose algorithm is fixed in the crate doc (splitmix64 and a
Fisher-Yates over the rank order), so the same snapshot, seed and N give the same bytes on any
host. "Same seed and date gives the same list" holds because the date names a committed snapshot,
not a live query.

Committed: `corpus/<language>/candidates.json` and `corpus/<language>/manifest.pinned.json` for
Go, Rust and Java. A fresh sample's manifest is made at run time and is not committed.

## Why

The loop must read the same repositories at the same commits twice to say a reader got better,
and must read new ones to find gaps. Star rankings move daily, so determinism has to come from a
snapshot, not from a rerun of the query.

## Done When

Failing tests first, all over a recorded `gh` output fixture (a stub `gh` written in POSIX sh
under `crates/corpus/tests/fixtures/`, prints canned JSON; no network). Each test is named with
its time in the report; none runs over 5 s, none over 60 s.

- `fetch` over the stub writes a snapshot equal to the checked-in `candidates.expected.json`;
  forks, archived and oversized repositories are absent and counted.
- `select --mode top --count 3` gives the first three by rank; `--mode random --seed 7 --count 3`
  gives the checked-in list, byte-equal on two runs and with the snapshot's rows in a different
  input order; seed 8 gives a different list.
- Near misses, each refused with a message naming the field and exit 1: a commit that is not 40
  lowercase hex; `--count` above the number of candidates; a candidate whose `language` differs
  from the snapshot's; a duplicate `name`; `--mode random` without `--seed`; `--seed` with
  `--mode top`; a manifest whose recorded snapshot digest differs from the file it names
  (`select --check <manifest> --candidates <file>` re-derives and compares).
- A `reader` id that `codespec read` has no adapter for (`--language Cobol`) is refused by name.
- `scripts/check-part.sh inventory` passes: the crate's test target is in a `test` part of
  `scripts/check-parts.txt`.
- The committed `corpus/go/manifest.pinned.json`, `corpus/rust/…`, `corpus/java/…` re-derive from
  their committed snapshots with `select --check`; that check is in the gate and reads no network.
- The three manifests are made once by a run with network, and the pull request says which date.
- `docs`: the crate doc holds both formats, the shuffle and every refusal; `AGENTS.md` table has
  the crate's row.

## Scope

- Own: `crates/corpus` (new: `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/snapshot.rs`,
  `src/select.rs`), `corpus/<language>/` (new), `Cargo.toml` (workspace member),
  `scripts/check-parts.txt`, `AGENTS.md`, `CHANGELOG.md`.
- Tools are Rust and POSIX sh; no Python. The stub `gh` is POSIX sh.
- Overlap, `story:java-reader-public-corpus` (draft): both pin Java repositories. This story does
  not touch `scripts/java-corpus.txt`, `scripts/read-corpus-java.sh` or `docs/java-corpus.md`. The
  pinned Java manifest here is the same six repositories at the same commits when that story
  merges first (its list is the input), or its own list when this one does; the later of the two
  re-points the other, and that re-pointing is a change to that story, which the owner decides.
- First story of the epic: it creates the crate, so every later story depends on it.
