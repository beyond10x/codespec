---
format: aep.planning-md/3
id: story:release-fact-diff-release
kind: story
status: draft
title: Release and install the codespec that ships the release fact diff
summary: The first codespec tag holding the three stories' merges, released with verified assets and installed on the workstation; codespec --version prints it
relations:
- decomposes: epic:release-fact-diff
- depends_on: story:stable-site-identity
- depends_on: story:go-http-entries
- depends_on: story:diff-command
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/main.go
- confidence: inferred
  path: adapters/go/cmd/codespec-write-go/main.go
- confidence: inferred
  path: adapters/groovy/src/codespec/read/groovy/Main.java
- confidence: inferred
  path: adapters/groovy/testdata
- confidence: inferred
  path: adapters/java/src/codespec/read/Main.java
- confidence: inferred
  path: adapters/java/testdata/quarkus.read.cir.json
- confidence: inferred
  path: adapters/odin/read/main.odin
- confidence: inferred
  path: conformance
- confidence: inferred
  path: crates/docs/src/status.rs
- confidence: inferred
  path: examples
- confidence: inferred
  path: website
- confidence: inferred
  path: website/docs/getting-started.md
- confidence: inferred
  path: website/docs/guides/add-an-adapter.md
revision: 2
---
# Story: Release and install the codespec that ships the release fact diff

## Outcome

A codespec release whose tag holds the merge commits of `story:stable-site-identity`,
`story:go-http-entries` and `story:diff-command` is published by the steps of `AGENTS.md`
§ Release, and its programs are installed on the workstation from the release assets: `codespec`
and `codespec-read-go` on the `PATH` of the workstation's shells, from the
`x86_64-unknown-linux-gnu` archive, with `SHA256SUMS` verified. The consumer then runs the
release diff with installed programs, not a build of a working tree.

`codespec --version` is wired: the clap `Cli` in `crates/cli/src/lib.rs` (line 17) declares
`version`, which prints `codespec <workspace version>`, and `website/docs/getting-started.md`
shows `codespec --version` printing `codespec 0.12.0`. This story adds nothing to it.

## Why

The consumer's ask C1: the release that ships the three stories is installed, and `codespec
--version` prints it. A merged story is not usable by the consumer until a release carries it and
the workstation runs that release.

## Observed

2026-11-25, on the workstation: `command -v codespec` prints nothing and exits 1; no `codespec`
is on the `PATH`, so there is no installed version to print. The workspace version on main
`5f82fff9` is 0.12.0 (`Cargo.toml` line 6).

## Done When

- Before: `command -v codespec` prints nothing and exits 1 (recorded above, 2026-11-25); if a
  codespec is installed by the time this story starts, `$(command -v codespec) --version` is run
  and its output recorded here first.
- The release tag `T` is the first codespec tag whose history holds all three merge commits:
  `git merge-base --is-ancestor <merge commit> T` exits 0 for each of the three, and exits 1 for
  at least one of them against every earlier tag.
- The release of `T` has its assets; `sha256sum -c SHA256SUMS` over the downloaded
  `codespec-T-x86_64-unknown-linux-gnu.tar.gz` passes, and the installed `codespec` and
  `codespec-read-go` are the files of that archive (`cmp` against the unpacked copy exits 0).
- After: `$(command -v codespec) --version` prints `codespec T`, and a document
  `codespec-read-go` writes carries `reader.version` `T`.
- With the installed programs, the consumer's checks of `epic:release-fact-diff` pass: the
  moved-lines fixture gives no delta, the syscall fixture gives its two deltas, the echo fixture
  gives its three `http` entries.
- The tag and the GitHub Release wait for the owner's approval (a publication); the version bump
  commit passes `task check` before the tag, as `AGENTS.md` § Release says.

## Scope

- Own: the release files `AGENTS.md` § Release lists for a version bump: `Cargo.toml`,
  `Cargo.lock`, `adapters/go/cmd/codespec-read-go/main.go`,
  `adapters/go/cmd/codespec-write-go/main.go`, `adapters/java/src/codespec/read/Main.java`,
  `adapters/groovy/src/codespec/read/groovy/Main.java`, `adapters/groovy/testdata/`,
  `adapters/java/testdata/quarkus.read.cir.json`, `adapters/odin/read/main.odin`,
  `crates/docs/src/status.rs`, `website/docs/getting-started.md`,
  `website/docs/guides/add-an-adapter.md`, `CHANGELOG.md`, and what `task examples`, `task
  conformance` and `task docs-generate` regenerate (`examples/`, `conformance/`, `website/`).
  Inferred; recorded as typed scope.
- After the three stories of the epic: `depends_on` edges. If another release already carries
  all three when this story starts, it only installs and verifies that release.
