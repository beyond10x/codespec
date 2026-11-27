---
format: aep.planning-md/3
id: story:barrier-language-matches-module
kind: story
status: draft
title: A barrier applies only to modules of its own language
summary: the default Go barrier no longer gives syscall rows to modules of another language
relations:
- decomposes: epic:release-diff-go-field-gaps
revision: 1
---
# Story: A barrier applies only to modules of its own language

## Outcome

`codespec diff` and `codespec effects` compare each summary's `language` with the module's
`reader.language`, but never the barrier's (`crates/effects/src/set.rs:308-330`; `main.rs` reads
no barrier language either). Since story:release-ships-go-summaries an installed release applies
the Go linux/amd64 barrier by default, so a module of another language whose extern carries a Go
import path (`syscall`) gets a Go syscall row: the adversary of that story edited a module to
`"language": "rust"` and still saw `syscall-added … syscall.Syscall@1`.

The change: a barrier is applied only to modules whose `reader.language` equals its `language`;
an explicit `--barrier` of another language than every module read is refused by name.

## Acceptance

- A Rust module with an extern `syscall.Syscall` and the Go barrier installed reports no syscall row.
- `codespec diff --barrier <go barrier>` over a set holding only Rust modules exits 2 naming both languages.

## Reported by

Adversary of story:release-ships-go-summaries, wave 6, 2026-11-26 (pre-existing; not taken by any
wave of the current plan).
