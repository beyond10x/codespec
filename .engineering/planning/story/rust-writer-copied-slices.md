---
format: aep.planning-md/3
id: story:rust-writer-copied-slices
kind: story
status: draft
title: The Rust writer writes a copy of a slice or map that is shared as in Go
summary: A copied slice, map or struct holding one is written in Rust with Go's sharing, or refused naming the copy where no form is exact.
relations:
- decomposes: epic:go-and-rust-writers-with-harness
revision: 2
---
# Story: The Rust writer writes a copy of a slice or map that is shared as in Go

## Outcome

A `cir/1` assignment, `let`, argument or result that copies a slice, a map, or a struct holding
one is written in Rust with Go's sharing: both names see the same elements (a shared,
reference-counted buffer the writer declares, or an index into one), so a later write through
either is seen by the other as in Go. Where no such form is exact, the writer refuses as today,
naming the copy.

## Why

A consumer that writes `cir/1` documents by hand from a specification (97 documents,
2026-11-24) lists among its tool limits: "`codespec-write-rust` refuses a copied slice", so
most of its decision functions cannot be written back to Rust. A function that takes a row with
a slice field and returns the row after is that shape. Reproduced on main `fff33d171` with a document
written from scratch (`Snapshot(xs []int64) []int64 { kept := xs; return kept }`, passed with
`codespec pass`): `codespec write --lang rust` writes no function and `refusals.json` holds "a
copy of a []int64, which shares a slice or map with the original in Go".

## Done When

- A failing test first in `adapters/rust/tests/`: `Snapshot`, and a function that copies a
  struct with a slice field and then writes an element through the copy, are written, build,
  and give Go's output under the harness; red on main.
- The crate doc's row for copies states the form.

## Scope

- `adapters/rust/src/write.rs`, `adapters/rust/tests/`, `CHANGELOG.md`. Inferred. Reproducer:
  `repro/tools/` (not committed).
