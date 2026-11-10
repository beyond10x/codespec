---
format: aep.planning-md/3
id: story:builtin-defect-checks
kind: story
status: implemented
title: Built-in defect checks over a linked set, tested on google/capslock
relations:
- decomposes: epic:codebase-transformations
- informed_by: story:defect-patterns
scope:
- confidence: cited
  path: crates/check
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: docs/checks-capslock.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-11-09T10:00:23Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: Built-in defect checks over a linked set, tested on google/capslock

## Outcome

`codespec check <check> <documents…>` (crate `crates/check`, command in `crates/cli`) runs one named
check over a set of `cir/1` documents and prints one finding per line: position, function, check
name, and the evidence the check read (the calls and positions it matched). `codespec check --list`
prints every check with one line on what it finds. `--format json` prints `codespec-check-report/1`.
Exit 0 with no findings, 1 with findings, 2 on an input error.

The checks are query kinds that hold for any codebase. They read only `cir/1`: bodies,
`foreign.calls`, opaque functions, method sets, `inits` and facts. No check reads source text.

| check | finds |
|---|---|
| `unbounded-growth` | a function on a receive path (`Read`, `ReadFrom`, `Accept` …) that inserts into a container keyed by input, with no remove reachable from the same receiver |
| `insecure-random` | a call of `math/rand` reachable from a function that also builds a token, key, nonce or id (by name), or any `math/rand` call when `--strict` |
| `int-from-text` | `strconv.Atoi`/`ParseInt`/`ParseUint` whose result is converted to a narrower integer type with no range comparison on the path |
| `case-fold-asymmetry` | sibling methods of one type (same receiver, names sharing a stem) where one compares names after case folding and another with `==` |
| `clone-aliasing` | sibling clone functions where one copies a slice field and another passes it through |
| `dropped-error` | a call of a fallible function whose error result is assigned to `_` or not used (`crates/passes` `fallible`) |
| `lock-without-unlock` | a `Lock`/`RLock` with a path to a `return` that does not pass the matching unlock or a deferred one |
| `unreached` | functions no entry reaches (wraps `codespec reach`) |

The method-name lists a check uses (receive, insert, remove, token names) are data in the check's
definition, printed by `--list`, and overridable with `--names <file>`.

`dropped-error` and `lock-without-unlock` are also named by `story:defect-patterns`. Here they are
Rust checks. `story:defect-patterns` is done when its pattern files match the same defects, so the
two stories do not claim the same deliverable. A check may later be rewritten as a pattern.

## Acceptance

- Each check has a fixture under `crates/check/tests/fixtures/<check>/`: one Go source with a true
  positive and one with a near miss that does not match, both read with `codespec-read-go`. A test
  asserts one finding on the first and none on the second.
- Tests assert exit 0 on a clean fixture, 1 on a finding, 2 on a missing document, and that
  `--format json` output parses as `codespec-check-report/1` with the same findings as the text
  output.
- `codespec check --list` prints 8 checks. The generated CLI reference page lists the command.
- The test repository is [google/capslock](https://github.com/google/capslock) (Apache-2.0), pinned
  to one commit. `scripts/read-repo.sh --deps` reads it with every package it imports, and every
  check runs over that set. The run is recorded in `docs/checks-capslock.md`: the commit, the
  document count, the time, and per check the finding count and every finding triaged by reading
  the capslock source as a true positive (with the line and why) or a false positive (with the
  reason the check matched). A true positive gets a reproducer test kept outside this repository
  and named in the record.
- From that triage, each check's false-positive rate is in the record. A check with more than half
  its findings false is narrowed and run again, or marked `experimental` in `--list`.
- No file this story adds or changes contains a name on the deny-list the coordinator holds outside
  the repository. The coordinator runs
  `git diff origin/main --name-only | xargs grep -i -F -f <deny-list>` before the merge, and it must
  print nothing.
- `task check` green.

## Out of Scope

- The pattern language (`story:cir-patterns`) and pattern files (`story:defect-patterns`).
- Taint flows (`story:taint-flows`) and unhandled branches (`story:unhandled-branches`).
- Checks over documents from the Rust, Java or Groovy readers: the checks read `cir/1`, but this
  story tests them on Go only.

## Shared surface and merge order

The operator asked (2026-11-09) for `story:c-reader-clang`, `story:c-writer`,
`story:builtin-defect-checks` and `story:use-cases-page` to run in one wave. They share these
files, each by appending one entry:

- `Cargo.toml` `[workspace] members` and `Cargo.lock`: `adapters/c-read`, `adapters/c`,
  `adapters/c-run`, `crates/check`.
- `.github/workflows/check.yml`: one test-matrix line per new crate.
- `Taskfile.yml`: the `adapters` task and the manifest loops (C stories only).
- `crates/docs/src/main.rs` `REPORTS` (C writer only), `crates/docs/src/support.rs` `LANGUAGES` and the generated
  `website/data/support-matrix.json` (C stories only).
- `crates/cli/src/lib.rs` `Cmd` (`story:builtin-defect-checks` only adds `check`; the C stories add
  no command).

Each implementor appends its own entries and touches no other story's. The coordinator merges in the
order `story:c-writer`, `story:c-reader-clang`, `story:builtin-defect-checks`,
`story:use-cases-page`, resolves the append conflicts in those files by keeping both entries, and
runs `task check` after each merge.

Generated files (`website/docs/reference/crates.md`, `website/docs/reference/cli.md`,
`website/data/support-matrix.json`, `website/data/conformance/*.json`) are never merged by hand:
the coordinator takes either side and runs `task docs-generate` after each merge.
`crates/docs/src/support.rs` `LANGUAGES` gains `c` in `story:c-writer` only. `story:c-reader-clang`
does not edit `LANGUAGES` or `REPORTS`.
