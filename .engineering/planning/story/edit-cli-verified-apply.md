---
format: aep.planning-md/3
id: story:edit-cli-verified-apply
kind: story
status: proposed
title: 'codespec edit: dry run, verification through an overlay, --apply'
summary: 'The command: read with spans, evaluate edit items, plan, verify by re-read, print the diff, apply'
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-go-reader
- depends_on: story:edit-spans-rust-reader
- depends_on: story:edit-cq-items
- depends_on: story:edit-engine-delete-retarget
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/cli/Cargo.toml
- confidence: inferred
  path: crates/cli/src/edit.rs
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/edit.rs
- confidence: inferred
  path: crates/cli/tests/fixtures
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
---
# Story: codespec edit: dry run, verification through an overlay, --apply

## Outcome

`codespec edit --lang <go|rust> <dir>… (-e <program> | -f <file>) [--apply] [--closed-world]
[--format text|json]` (`docs/code-edit-language.md` § 3.3):

1. reads each directory with the reader's `--spans` into one set and its span documents
   (`story:edit-spans-format`), refusing a language whose reader gives no spans;
2. builds the fact base with `source_name` and `extent` from the spans, evaluates the program's
   `edit` items (`story:edit-cq-items`) and refuses every possible row;
3. builds the change set with `crates/edit`;
4. writes the edited files' contents under the cache directory (never beside the inputs), reads
   every changed module again with `--overlay` and `--spans`, and compares with `S'`;
5. prints the diff (text) or `codespec-edit-report/1` (json: rows with status, files with SHA-256
   before and after, the diff; no time field);
6. with `--apply`, after verification and only if every file still has the SHA-256 it was read
   with, writes each file through a temporary file in its directory and a rename.

Exit 0, 1 (a refusal or a failed verification, nothing written, each reason printed), 2 (an error).

## Done When

- `crates/cli/tests/edit.rs`, each case on a copy of a small Go fixture (two packages) or Rust fixture
  under `crates/cli/tests/fixtures/edit/`, with the readers built once for the run as the existing
  CLI tests get their adapters:
  - a rename across both Go packages: the dry-run diff equals the checked-in one and no file
    changes (hashes compared); `--apply` writes exactly the diff's files, and a second run of the
    same program prints nothing and exits 0;
  - the same rename on the Rust fixture edits `load`, not `Load`;
  - a rename whose target is called inside a `foreign` statement: exit 1 naming its position, no
    file written;
  - a verification failure, forced by a fixture reader that drops a node on re-read: exit 1 with
    the first differing path, no file written;
  - a file changed between read and `--apply`: exit 1, no file written;
  - `--format json` on the same input twice: byte-identical.
- `task docs-generate` run; `website/docs/reference/cli.md` current; `CHANGELOG.md` entry.
- No test over 5 s; the `test b10x-codespec-cli` part stays under 120 s; no Python.

## Scope

- Own: `crates/cli/src/edit.rs` (new), `crates/cli/src/lib.rs` (`Cmd::Edit`), `crates/cli/src/main.rs`,
  `crates/cli/Cargo.toml` (depends on `b10x-codespec-edit`), `crates/cli/tests/edit.rs`,
  `crates/cli/tests/fixtures/edit/`, `website/docs/reference/cli.md` (generated), `CHANGELOG.md`.
- Runs after `story:edit-spans-format` (same `crates/cli` files and `CHANGELOG.md`),
  `story:edit-spans-go-reader`, `story:edit-spans-rust-reader`, `story:edit-cq-items` and
  `story:edit-engine-delete-retarget`.
