---
format: aep.planning-md/3
id: story:edit-cli-verified-apply
kind: story
status: proposed
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
  path: website/docs/reference/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
---
# Story: codespec edit: Go, every wave-1 operation, --apply, the JSON report

## Outcome

`codespec edit` (`docs/code-edit-language.md` § 3.3), built on `story:edit-slice-rust-rename`, takes
`--lang go` (the copy is the directory of the `go.mod` above the inputs, a relative `replace` path
rewritten to absolute) and every wave-1 operation (`retarget`, `delete` from
`story:edit-engine-delete-retarget`), and adds:

- `--apply`: after verification, and only if every file still has the SHA-256 it was read with,
  each file is written through a temporary file in its directory and a rename;
- `--format json`: `codespec-edit-report/1` (rows with status and reason, files with SHA-256 before
  and after, the diff; no time field);
- refusal of a language whose reader gives no spans, naming the reader.

## Done When

`crates/cli/tests/edit/mod.rs`, each case on a copy of a small Go fixture (two packages, under
`crates/cli/tests/fixtures/edit/go/`) or the Rust fixture, readers built once for the run:

- a rename across both Go packages with `--closed-world`: the dry-run diff equals the checked-in one
  and no input file changes (hashes compared); `--apply` writes exactly the diff's files, and a
  second run of the same program prints nothing and exits 0;
- `--closed-world`: the same rename without it exits 1 naming the exported target and writes
  nothing;
- retarget and delete on the Go fixture: the checked-in diffs, applied and verified;
- a rename whose target is called inside a `foreign` statement: exit 1 naming its position, no file
  written;
- a file changed between read and `--apply`: exit 1, no file written;
- exit 2: a program that does not parse; an input directory that does not exist;
- `--lang java` with the Java reader: exit 1 "the java reader gives no spans";
- `--format json` on the same input twice: byte-identical.

`task docs-generate` run; `website/docs/reference/cli.md` current; `CHANGELOG.md` entry. No test
over 5 s; the `test b10x-codespec-cli` part stays under 120 s; no Python.

## Critic findings answered

- `--closed-world`, exit 2 and the no-spans refusal untested (acceptance, blocker): the cases above.
- A new test binary (parallel safety): the module of the existing `query.rs` target.

## Scope

- Own: `crates/cli/src/edit.rs`, `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`,
  `crates/cli/tests/edit/mod.rs`, `crates/cli/tests/fixtures/edit/`,
  `website/docs/reference/cli.md` (generated), `CHANGELOG.md`.
- Runs after `story:edit-slice-rust-rename` (same files), `story:edit-spans-go-reader` (held, see
  its story) and `story:edit-engine-delete-retarget`.
