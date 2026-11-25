---
format: aep.planning-md/3
id: story:rules-runner
kind: story
status: draft
title: 'codespec rules run: language filter, per-rule bounds, batches and test-file exclusion'
summary: A Rust command that runs the active rules over cir/1 documents with not-applicable reporting, time and memory bounds per rule, batches and test-file exclusion; run outputs stay with the caller.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-layout
- depends_on: story:rules-third-party-attribution
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/rules/src
- confidence: inferred
  path: crates/rules/tests/main.rs
- confidence: inferred
  path: docs/rules.md
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 4
---
# Story: codespec rules run: language filter, per-rule bounds, batches and test-file exclusion

## Outcome

`codespec rules run --docs <dir> [--category c] [--rules <id>…] [--names <dir>] --out <dir>` (Rust,
`crates/rules`, no Python, no script that decides a row) runs the active rules over a set of
`cir/1` documents and writes one rows file per rule and a summary:

- A rule whose `languages` holds none of the documents' languages is not run and is reported as
  `not applicable`, never as 0 rows.
- Each rule runs under a wall-clock bound and a memory bound (`--rule-timeout`, default 300 s;
  `--rule-memory`, default 4 GiB). A rule over a bound is recorded as `limit: time` or `limit:
  memory` and the run continues with the next rule; the run exits non-zero when any rule failed and
  names each.
- Documents are queried in batches (`--batch <n>`, default 500) for rules whose rows never cross a
  document; a rule that follows calls across documents runs over the whole set or is reported as
  `too large` under its bound.
- Test code is left out by default (`tests/`, `*_test.go`, `*_tests.rs`, `test_support`,
  `testing/`); `--include-tests` keeps it.
- Names lists merge in this order: the rule's defaults, the run's `module`, then one
  `<rule>.json` per rule from `--names`.

What stays private: `--names`, `--docs` and `--out` are paths the caller gives; nothing of a run is
written into this repository, and no run over a code base other than a fixture or the public corpus
is committed. A run over a private code base, its names lists and its rows live where the caller
keeps them (`story:codesec-retirement`).

The two open codesec issues this answers: a performance rule that grew to 33 GB over 5,087 Rust
documents with no bound (codesec issue 13), and the runner items of codesec issue 14 (no language
filter, the first failing rule ending its category, no per-rule bound, test files not left out).

## Why

The runner is codesec's `Taskfile.yml` `hunt` task and its two Rust-repository scan scripts (copy a
pinned commit, read every file, run the hunt per category, split the rows per rule); this command
replaces all three, and the scan scripts are not moved. Moving it as a command makes
it testable, bounded and usable by the reader corpus loop and by any caller, without the private
repository.

## Done When

Failing tests first, in `crates/rules/tests/main.rs`, over committed made-up fixture documents (no
reader run, no network); within 60 s each:

- a Go-only rule over Rust documents prints `not applicable` and writes no rows file;
- a made-up rule that recurses past a 1 s `--rule-timeout` is recorded as `limit: time`, the next
  rule still runs, the exit code is non-zero and the summary names the rule;
- the memory bound is set on the query process (tested by a made-up rule and a 64 MiB bound; it is
  recorded as `limit: memory`);
- rows from `--batch 1` equal rows from one batch for a per-document rule;
- a file `foo_tests.rs` is left out and its rows are absent; with `--include-tests` they are there;
- a `--names` file for one rule changes that rule's rows only;
- merge order: a list set in the rule's defaults, in `module` and in the rule's `--names` file
  takes the `--names` value; set in the defaults and `module` only, it takes `module`'s; each case
  is a row that exists under one value and not the other;
- the summary (`<out>/summary.json`) holds per rule: status (`ran`, `not applicable`, `limit: time`,
  `limit: memory`, `too large`), rows, and rows inside a read body versus inside a refused one; it
  is the interface `story:rules-climb-lane` and the reader corpus loop read, and a test pins its
  keys.

## Scope

`crates/rules/src/` (a run module), `crates/rules/tests/main.rs`, `crates/cli/src/lib.rs`,
`docs/rules.md` (§ running), the generated `website/docs/reference/cli.md`, `CHANGELOG.md`.
Depends on `story:rules-layout` and `story:rules-third-party-attribution`: it shares the crate, its
test file and `docs/rules.md` with that story and `story:rules-sanitization-gate` and lands after
both. `story:rules-sanitize-and-fixtures` lands after this one (shared `CHANGELOG.md`,
`docs/rules.md`).
