---
format: aep.planning-md/3
id: story:writer-zig
kind: story
status: draft
title: A Zig writer for the functional core
summary: codespec-write-zig writes structs, maps, slices, integers, strings, if, for and switch, and refuses the rest.
revision: 2
---
# Story: a Zig writer for the functional core

## Outcome

`codespec-write-zig` writes a `cir/1` module as Zig for the functional-core subset: structs, maps,
slices, integers, strings, `if`, `for` and `switch`. It refuses everything else at a position with
a reason, as the other writers do, and its conformance report shows what it covers.

## Why

Requested on 2026-11-11 by another session that generates small servers through `cir/1` and
writes them in Go, Rust, Zig and one more language; Zig has no writer.

## Acceptance

- `codespec-write-zig` exists beside the Go, Rust and C writers, with a runner the conform engine
  can call, and `codespec conform` runs the kit against it.
- Every kit case of the subset above passes; every other case is a refusal with a reason, and the
  support matrix lists the writer.
- Allocation and ownership of maps, slices and strings are written one way, stated in the writer's
  README.
- The release build ships the writer for the three targets; `task check` exits 0.
