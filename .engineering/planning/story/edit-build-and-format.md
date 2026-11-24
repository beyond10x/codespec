---
format: aep.planning-md/3
id: story:edit-build-and-format
kind: story
status: implemented
title: codespec edit --build, and gofmt of edited Go files
summary: gofmt a gofmt-clean Go file after the edit; --build compiles the verified change before writing
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/edit.rs
- confidence: inferred
  path: crates/cli/tests/edit
- confidence: inferred
  path: crates/cli/tests/fixtures/edit
- confidence: inferred
  path: crates/edit/src/change.rs
- confidence: inferred
  path: crates/edit/src/post.rs
- confidence: inferred
  path: crates/edit/src/run.rs
- confidence: inferred
  path: crates/edit/tests/main.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:48Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-24T07:36:21Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
# Story: codespec edit --build, and gofmt of edited Go files

## Outcome

- gofmt as merged text edits: after the span edits, a Go file that was `gofmt`-clean when read is
  run through `gofmt`. The formatted text and the span-edited text must have the same characters
  outside whitespace (checked); walking both in step, each whitespace run that differs becomes one
  text edit on the original file, merged into the change set with the span edits (a run touching a
  span edit joins it). So the diff stays derived from edits (`docs/code-edit-language.md` § 8) and
  `diff.rs` needs no text-diff mode. A Go file that was not `gofmt`-clean and every Rust file keep
  the span-edited text. Chosen over a text-diff mode in `diff.rs` because it is simpler and keeps one
  source of truth for the diff and the written bytes.
- `--build` compiles the verified copy (§ 7.4) before anything is written: Go with `go build ./...`
  and `go vet ./...` (test files included) in the copy of the `go.mod` root; Rust with
  `cargo check --all-targets` in the copy of the workspace root (`story:edit-slice-rust-rename`).
  `go` and `cargo` run through the host's `slot` shims; each call is bounded at 10 minutes. A
  failure exits 1 with the compiler's first 20 lines and writes nothing.

## Done When

`crates/edit/tests/main.rs`: the whitespace alignment of a gofmt-realigned struct gives the expected
merged edits; texts that differ outside whitespace are refused.

`crates/cli/tests/edit/mod.rs`:

- renaming a field of a gofmt-aligned struct gives a diff whose struct is realigned and `gofmt -l`
  clean, equal to the checked-in diff; a non-gofmt-clean file keeps its spacing;
- `--build` on the Go fixture where a `_test.go` file calls the function being deleted: exit 1
  naming the test file, nothing written; without `--build` the same delete is planned (the reader
  does not read test files), as § 7.2 says;
- `--build` on the Rust fixture where a `#[cfg(test)]` module calls the function being deleted:
  exit 1 naming that file, nothing written; a Rust rename with `--build`: exit 0;
- `--build` of a rename in a Rust workspace member (`version.workspace = true`, a path dependency on
  a sibling member): exit 0, the copy holding the workspace root.

No test over 60 s (the build cases are the slowest; each one's time is printed by the test); the
`test b10x-codespec-cli` part stays under 120 s.

## Critic findings answered

- Round 1, the Rust `--build` case could not fail: the `#[cfg(test)]` delete case.
- Round 2, `cargo check` in a crate copy fails for a workspace member: the copy holds the workspace
  root; the workspace-member case.
- Round 2, "the diff shows the formatted text" against a diff built from edits: gofmt's changes are
  merged text edits, decided and tested above.

## Scope

- Own: `crates/edit/src/post.rs` (new: gofmt alignment and build commands), `crates/edit/src/change.rs`
  (merging the whitespace edits), `crates/edit/src/run.rs`, `crates/edit/tests/main.rs`,
  `crates/cli/src/edit.rs` (the `--build` flag), `crates/cli/tests/edit/mod.rs`,
  `crates/cli/tests/fixtures/edit/`, `CHANGELOG.md`.
- Runs after `story:edit-cli-verified-apply` (same files).
