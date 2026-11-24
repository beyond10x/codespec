---
format: aep.planning-md/3
id: story:edit-cli-verified-apply
kind: story
status: implemented
title: 'codespec edit: Go, every wave-1 operation, --apply, the JSON report'
summary: 'Extends the Rust slice: Go, retarget and delete, --apply with a stale check, codespec-edit-report/1'
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-go-reader
- depends_on: story:edit-engine-delete-retarget
- depends_on: story:edit-slice-rust-rename
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/edit.rs
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/edit
- confidence: inferred
  path: crates/cli/tests/fixtures/edit
- confidence: inferred
  path: crates/edit/src/run.rs
- confidence: inferred
  path: crates/edit/tests/main.rs
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:47Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-24T07:16:23Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
# Story: codespec edit: Go, every wave-1 operation, --apply, the JSON report

## Outcome

`codespec_edit::run`, built in `story:edit-slice-rust-rename`, takes Go (the copy is the directory of
the `go.mod` above the inputs; a relative `replace` path in the copy's `go.mod` is rewritten to the
absolute path of its target) and every wave-1 operation (`retarget`, `delete` from
`story:edit-engine-delete-retarget`), and adds:

- apply: after verification, and only if every file still has the SHA-256 it was read with, each
  file is written through a temporary file in its directory and a rename;
- `codespec-edit-report/1`: rows with status (`planned`, `applied`, `refused`) and reason, files
  with SHA-256 before and after, the diff; no time field;
- refusal of a language whose reader gives no spans, naming the reader.

`codespec edit` stays a thin wrapper: `--apply` and `--format json` map to the library's options.

## Done When

`crates/cli/tests/edit/mod.rs`, each case on a copy of a small Go fixture (two packages, under
`crates/cli/tests/fixtures/edit/go/`) or the Rust fixture, readers built once for the run:

- a rename across both Go packages with `--closed-world`: the dry-run diff equals the checked-in one
  and no input file changes (hashes compared); `--apply` writes exactly the diff's files, and a
  second run of the same program prints nothing and exits 0;
- `--closed-world`: the same rename without it exits 1 naming the exported target and writes
  nothing;
- retarget and delete on the Go fixture: the checked-in diffs, applied and verified;
- a Go module whose `go.mod` has `replace example.com/lib => ../lib` (a sibling module in the
  fixture) and calls `lib`: a rename in the module verifies (without the rewrite the copy's
  `../lib` names a directory that does not exist in the cache and the re-read fails); in
  `crates/edit/tests/main.rs` the `go.mod` rewrite turns `=> ../lib` into the absolute path, keeps an
  absolute or module-path `replace` as it is, and keeps every other byte;
- a rename whose target is called inside a `foreign` statement: exit 1 naming its position, no file
  written;
- a file changed between read and `--apply`: exit 1, no file written;
- exit 2: a program that does not parse; an input directory that does not exist;
- a language whose reader gives no spans: an adapter directory (`--adapters`) whose
  `codespec-read-java` is a POSIX sh fixture printing a bare module; `--lang java` exits 1 with "the
  java reader gives no spans" (no JDK is used);
- `--format json` on a fixture with one planned and one refused row equals a checked-in
  `codespec-edit-report/1` file (both statuses, the reason, both hashes per file, the diff), and a
  second run is byte-identical.

`task docs-generate` run; `website/docs/reference/cli.md` current; `CHANGELOG.md` entry. No test
over 5 s; the `test b10x-codespec-cli` part stays under 120 s; no Python.

## Critic findings answered

- Round 1, `--closed-world`, exit 2 and the no-spans refusal untested: the cases above.
- Round 2, the relative `replace` rewrite has no failing case: the `replace` case and the unit cases
  of the rewrite.
- Round 2, the report's fields only tested as byte-identical: the checked-in report.
- Round 2, `--lang java` needs a JVM: the sh fixture reader; no JDK.
- Round 2, orchestration in the CLI: it is `codespec_edit::run`; the CLI maps flags.

## Scope

- Own: `crates/edit/src/run.rs`, `crates/edit/tests/main.rs`, `crates/cli/src/edit.rs`, `crates/cli/src/lib.rs`,
  `crates/cli/src/main.rs`, `crates/cli/tests/edit/mod.rs`, `crates/cli/tests/fixtures/edit/`,
  `website/docs/reference/cli.md` (generated), `CHANGELOG.md`.
- Runs after `story:edit-slice-rust-rename` (same files), `story:edit-spans-go-reader` (held, see
  its story) and `story:edit-engine-delete-retarget`.
