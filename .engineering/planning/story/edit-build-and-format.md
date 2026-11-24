---
format: aep.planning-md/3
id: story:edit-build-and-format
kind: story
status: proposed
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
  path: crates/edit/src/post.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
---
# Story: codespec edit --build, and gofmt of edited Go files

## Outcome

- After the edits and before verification, a Go file that was `gofmt`-clean when read is run
  through `gofmt`; the result must differ from the span-edited text only in whitespace (checked),
  and the diff shows the formatted text. A Go file that was not `gofmt`-clean and every Rust file
  keep the span-edited text (`docs/code-edit-language.md` § 6.1).
- `--build` compiles the verified copy (§ 7.4) before anything is written: Go with `go build ./...`
  and `go vet ./...` (test files included); Rust with `cargo check --all-targets`. `go` and `cargo`
  run through the host's `slot` shims; each call is bounded at 10 minutes. A failure exits 1 with
  the compiler's first 20 lines and writes nothing.

## Done When

`crates/cli/tests/edit/mod.rs`:

- renaming a field of a gofmt-aligned struct gives a diff whose struct is realigned and `gofmt -l`
  clean; a non-gofmt-clean file keeps its spacing;
- `--build` on the Go fixture where a `_test.go` file calls the function being deleted: exit 1
  naming the test file, nothing written; without `--build` the same delete is planned (the reader
  does not read test files), as § 7.2 says;
- `--build` on the Rust fixture where a `#[cfg(test)]` module calls the function being deleted:
  exit 1 naming that file, nothing written; a Rust rename with `--build`: exit 0.

No test over 60 s (the build cases are the slowest; each is timed in the PR); the
`test b10x-codespec-cli` part stays under 120 s.

## Critic findings answered

- The only Rust `--build` case cannot fail (acceptance, blocker): the `#[cfg(test)]` delete case.

## Scope

- Own: `crates/edit/src/post.rs` (new: formatting and build commands as data),
  `crates/cli/src/edit.rs`, `crates/cli/tests/edit/mod.rs`, `crates/cli/tests/fixtures/edit/`,
  `CHANGELOG.md`.
- Runs after `story:edit-cli-verified-apply` (same files).
