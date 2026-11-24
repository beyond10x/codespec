---
format: aep.planning-md/3
id: story:edit-slice-rust-rename
kind: story
status: implemented
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
  path: crates/edit/src/run.rs
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T01:01:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:46Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-24T03:39:04Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: First end-to-end edit: a Rust rename, dry run, verified by re-read

## Outcome

`codespec_edit::run` (the module of `crates/edit` that starts processes and touches files,
`story:edit-spans-format`) gains the pipeline, and `codespec edit --lang rust [--adapters <dir>]
[--closed-world] <dir>… (-e <program> | -f <file>)` is a thin call of it in `crates/cli`, so
`crates/mcp` can call the same pipeline later. For `rename` and `rename_field`, as a dry run:

1. reads each directory with the Rust reader's `--spans` (`run::read_spanned`,
   `story:edit-spans-rust-reader`);
2. builds the fact base with `source_name` (`ids::source_names`, `FactBase::build_with_names` of
   `story:edit-cq-items`) and evaluates the `edit` items, refusing every possible row with its sites;
3. builds the change set (`story:edit-engine-rename`);
4. copies the root the readers and the build resolve against (the Cargo workspace root: the nearest
   directory above the crate whose `Cargo.toml` has `[workspace]`, else the crate directory) without
   `.git` and `target` into the cache directory (never beside the inputs), writes the edited files
   into the copy, reads the copied input directories again with `--spans` and compares with `S'`
   (`docs/code-edit-language.md` § 7.3);
5. prints the diff; writes nothing to the inputs.

Exit 0 (planned and verified, or no row), 1 (a refusal or a failed verification, each reason
printed), 2 (an error). Until `story:edit-cli-verified-apply`, `--lang go` and the operations
`retarget` and `delete` exit 2 with "not yet: <what>".

## Done When

`crates/cli/tests/edit/mod.rs` (the module of the existing `query.rs` target), on a Rust fixture
crate under `crates/cli/tests/fixtures/edit/rust/` with the real Rust reader built once for the run
as the existing CLI tests get their adapters:

- renaming `pub fn load` to `fetch_all` with `--closed-world`: the diff equals the checked-in one,
  edits `load` and its `use` path, and every input file's SHA-256 is unchanged after the run;
- the same program without `--closed-world`: exit 1 naming the exported target;
- `rename_field` of a field with a shorthand literal: the checked-in diff;
- a possible row: `edit rename(F, "gone") :- func{id: F, name: "Helper"}, !calls(_, F).` over a
  fixture where a function value is called (an unknown site): exit 1 naming the site, nothing
  planned;
- a program whose body has no row: exit 0, empty output;
- `--lang go`, and a `delete` program on the Rust fixture: each exit 2 with "not yet";
- a crate that is a member of a Cargo workspace (`version.workspace = true` and a path dependency on
  a sibling member): the rename verifies, the copy holding the workspace root;
- verification failure: a fixture reader (a POSIX sh script calling the real reader and deleting
  one function from its output when it reads under the cache directory) gives exit 1 with the first
  differing path, and the inputs are unchanged;
- the time of the copy and the re-read is printed by the test; the case stays under 5 s.

Same module: `codespec_edit::run` called as a library over the same fixture gives the same diff as
the CLI. No test over 5 s; the `test b10x-codespec-cli` part stays under 120 s; no Python.

## Critic findings answered

- Round 1, real reader output first meets the engine at the full command: this slice comes first.
- Round 2, possible rows, "not yet" and the no-row exit untested: a case each above.
- Round 2, the pipeline in `crates/cli/src/edit.rs` where `crates/mcp` cannot reach it: it is
  `codespec_edit::run`; `crates/cli/src/edit.rs` only parses flags and prints.
- Round 2, Rust copies only the crate: the copy holds the workspace root; the workspace-member case.

## Scope

- Own: `crates/edit/src/run.rs`, `crates/cli/src/edit.rs` (new, thin),
  `crates/cli/src/lib.rs` (`Cmd::Edit`), `crates/cli/src/main.rs`, `crates/cli/tests/edit/mod.rs`,
  `crates/cli/tests/fixtures/edit/rust/`, `website/docs/reference/cli.md` (generated),
  `CHANGELOG.md`.
- Runs after `story:edit-spans-rust-reader`, `story:edit-cq-items` and `story:edit-engine-rename`.
  Beside `story:edit-engine-delete-retarget`: that story edits `crates/edit/src/lib.rs`, `retarget.rs`
  and `delete.rs`, this one `run.rs` (declared by `story:edit-spans-format`, so `lib.rs` is not
  touched here), and puts its tests in `crates/cli`, not in `crates/edit/tests`: no shared file. `story:edit-cli-verified-apply` extends the same files after it.
