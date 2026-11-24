---
format: aep.planning-md/3
id: story:go-writer-partial-module
kind: story
status: draft
title: The Go writer writes a module with refused functions left out and listed
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: The Go writer writes a module with refused functions left out and listed

## Outcome

With an option, the Go writer writes every function it can and leaves out each function holding
a construct it cannot write (a `foreign` statement of another language among them), listing each
in a refusals file with its position and reason, as the Rust writer does; what calls a left-out
function is left out too and listed. Without the option the output and exit code are as today.

## Why

`adapters/go/cmd/codespec-write-go/main.go` (doc): "A construct Go cannot hold is refused: exit
1 and a message naming its position, and nothing is written." A Rust module with one log macro
or one spawn gives `foreign statement in "rust" cannot be written as Go (…)` and no Go at all
(probes: three of five modules). A translation that grows function by function needs the part
that can be written.

## Acceptance

- A writer test: a module with one function holding a Rust `foreign` statement and one caller of
  it is written with both left out and listed; the rest builds and `go vet` passes.
- Without the option, every existing writer test and example output is byte-equal.

## Scope

- `adapters/go/cmd/codespec-write-go/`, `crates/cli/src/lib.rs` (passing the option),
  `CHANGELOG.md`. Inferred.
