---
format: aep.planning-md/3
id: story:release-ships-go-summaries
kind: story
status: implemented
title: The release archive ships the Go summaries and the syscall barrier
summary: codespec diff from an installed release alone resolves time.Now to its effect
relations:
- decomposes: epic:release-diff-go-field-gaps
- informed_by: story:go-std-effect-summaries-common
- depends_on: story:go-std-effect-summaries-common
- depends_on: story:diff-unknowns-summary
scope:
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/adversary_kit_release.rs
- confidence: inferred
  path: crates/cli/tests/effects.rs
- confidence: inferred
  path: crates/cli/tests/fixtures
- confidence: inferred
  path: docs/static-analysis.md
- confidence: inferred
  path: website/docs/concepts/static-analysis.md
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T15:11:23Z", actor: "human:operator", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-26T15:11:23Z", actor: "human:operator", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-26T18:43:26Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The release archive ships the Go summaries and the syscall barrier

## Outcome

A consumer who installs only a release archive gets the Go standard-library effect summaries and
the linux/amd64 syscall barrier at the release's version. Today
`summaries/go-std.effects.json` and `summaries/go-linux-amd64.barrier.json` are in neither the
platform archives (the Package step of `.github/workflows/release-build.yml:100-112` names every
archived file in its `tar -czf` line, and no summary is among them) nor the conformance archive
(`task conformance-kit`, `Taskfile.yml:497-525`), and the `codespec` binary has none built in.
`codespec diff` and `codespec effects` take them only as `--summaries` and `--barrier` files, so
a release consumer either runs without them or builds them from an unpinned checkout.

The change:

- The Package step puts `summaries/*.json` into every platform archive under `share/codespec/summaries/` (beside the binary: the archive is flat), and `task conformance-kit` puts them into the conformance archive the same way.
- A check that fails when either file is missing from an archive: a workflow step over `tar -tzf`, and the local Rust test that reads the Package step's tar line (`crates/cli/tests/adversary_kit_release.rs:24-33`).
- `codespec diff` and `codespec effects` read `<directory of the executable>/share/codespec/summaries/` when no `--summaries` / `--barrier` is given. An explicit flag replaces the default; `--no-default-summaries` turns it off. `reach` reads no summaries or barrier today (`crates/cli/src/main.rs:385-398`) and is out of this story.
- The lookup never falls back to the source tree, so the tests that run `target/debug/codespec` without flags (`crates/cli/tests/effects.rs:44-59`) keep their output.
- `website/docs/getting-started.md` (Install) says where the files are and that they carry the release's version; `cli.md` is regenerated.

## Acceptance

- From a temporary prefix holding the `codespec` binary and `share/codespec/summaries/` copied from the tree, with no flag, `codespec diff` over two revisions of a synthetic Go module that adds a call to `time.Now` reports the call's `time` effect, not `extern without summary`.
- `codespec diff --no-default-summaries` over the same pair reports `extern without summary` for it.
- The archive check fails when either file is missing from an archive.

`time.Now` enters `summaries/go-std.effects.json` with story:go-std-effect-summaries-common; this
story depends on it.

## Reported by

A consumer moving its code facts onto the 0.14.0 release (gap G5, relayed by the host operator,
2026-11-26).
