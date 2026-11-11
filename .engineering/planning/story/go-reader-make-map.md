---
format: aep.planning-md/3
id: story:go-reader-make-map
kind: story
status: draft
title: The Go reader reads make of a map
summary: make(map[K]V) and make(map[K]V, n) are read as an empty map, where they are refused today.
revision: 2
---
# Story: the Go reader reads `make` of a map

## Outcome

`codespec-read-go` reads `make(map[K]V)` and `make(map[K]V, n)` as an empty map of that type,
where today it refuses the expression as "expression outside cir/1".

## Why

Requested on 2026-11-11 by another session that writes a runtime library in plain Go and reads it
into `cir/1`; without `make` of a map the library cannot be read. A map literal `map[K]V{}` is read
today, so `cir/1` has the value; the reader lacks only this form of it.

## Acceptance

- A made-up Go file with `make(map[string]int)`, `make(map[string][]int, 8)` and a map built in a
  loop is read without a refusal; the capacity argument is evaluated for its effects and dropped.
- The document is the one the same file gives with `map[K]V{}` in place of each `make`, apart from
  positions.
- `make` of a slice and of a channel keep their present behaviour; `task check` exits 0.
