---
format: aep.planning-md/3
id: story:edit-slice-rust-rename
kind: story
status: proposed
title: 'First end-to-end edit: a Rust rename, dry run, verified by re-read'
summary: codespec edit for Rust rename and rename_field, dry run, re-read of a cache copy compared with the expected cir/1
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-rust-reader
- depends_on: story:edit-cq-items
- depends_on: story:edit-engine-rename
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
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T01:01:55Z", actor: "human:timo", revision: 3}
---
# Story: First end-to-end edit: a Rust rename, dry run, verified by re-read

## Outcome

`codespec edit --lang rust [--adapters <dir>] [--closed-world] <dir>… (-e <program> | -f <file>)`
runs end to end for `rename` and `rename_field` as a dry run, so real reader output meets the
engine before the full command:

1. reads each directory with the Rust reader's `--spans` (`story:edit-spans-format`,
   `story:edit-spans-rust-reader`);
2. builds the fact base with `source_name` from the spans and evaluates the `edit` items
   (`story:edit-cq-items`), refusing every possible row;
3. builds the change set (`story:edit-engine-rename`);
4. copies the crate directory without `.git` and `target` into the cache directory (never beside
   the inputs), writes the edited files into the copy, reads the copy again with `--spans` and
   compares with `S'` (`docs/code-edit-language.md` § 7.3);
5. prints the diff; writes nothing to the inputs.

Exit 0 (planned and verified, or no row), 1 (a refusal or a failed verification, each reason
printed), 2 (an error). Other languages and operations are refused with "not yet" until
`story:edit-cli-verified-apply`.

## Done When

`crates/cli/tests/edit/mod.rs` (the module of the existing `query.rs` target), on a Rust fixture
crate under `crates/cli/tests/fixtures/edit/rust/` with the real Rust reader built once for the run
as the existing CLI tests get their adapters:

- renaming `pub fn load` to `fetch_all` with `--closed-world`: the diff equals the checked-in one,
  edits `load` and its `use` path, and every input file's SHA-256 is unchanged after the run;
- the same program without `--closed-world`: exit 1 naming the exported target;
- `rename_field` of a field with a shorthand literal: the checked-in diff;
- verification failure: a fixture reader (a POSIX sh script calling the real reader and deleting
  one function from its output when it reads under the cache directory) gives exit 1 with the first
  differing path, and the inputs are unchanged;
- the test prints the time of the copy and the re-read; it is under 5 s on the fixture and is stated
  in the PR.

No test over 5 s; the `test b10x-codespec-cli` part stays under 120 s; no Python.

## Critic findings answered

- Real reader output first meets the engine at the full command (design, note): this slice comes
  first, on Rust because the Go reader is held.

## Scope

- Own: `crates/cli/src/edit.rs` (new), `crates/cli/src/lib.rs` (`Cmd::Edit`),
  `crates/cli/src/main.rs`, `crates/cli/tests/edit/mod.rs`, `crates/cli/tests/fixtures/edit/rust/`,
  `website/docs/reference/cli.md` (generated), `CHANGELOG.md`.
- Runs after `story:edit-spans-rust-reader`, `story:edit-cq-items` and `story:edit-engine-rename`;
  `story:edit-cli-verified-apply` extends the same files after it.
