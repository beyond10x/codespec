---
format: aep.planning-md/3
id: story:go-reader-if-init-error-check
kind: story
status: draft
title: The Go reader reads an error check written in the if's init statement
revision: 1
---
# Story: The Go reader reads an error check written in the if's init statement

## Outcome

`codespec-read-go` reads `if err := f(); err != nil { return <zero values>, err }`, the most common
Go spelling of a propagated error, as the same `cir/1` it gives the two-statement form
`err := f(); if err != nil { … }`, so the Rust writer can write it with `?` instead of the reader
keeping it as `foreign`.

## Context

Found by the adversary review of story rust-writer-error-propagation (2026-11-07): `front.go`
keeps an `if` with an init statement as `foreign` with a refusal, so a Go source written in the
idiomatic form never reaches the writer's propagation path. Already true before that story.

## Acceptance

- A test in `adapters/go/cmd/codespec-read-go/` reads `if err := f(); err != nil { return 0, err }`
  with no refusal, and the module equals the one read from the two-statement form, the scope of
  `err` kept to the `if`.
- Every checked-in `*.read.cir.json` and `case.cir.json` is byte-unchanged, or the change is
  explained per file.
