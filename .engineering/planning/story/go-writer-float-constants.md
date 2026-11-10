---
format: aep.planning-md/3
id: story:go-writer-float-constants
kind: story
status: draft
title: The Go writer evaluates float constants as binary64
summary: Float literal arithmetic and -0.0 mean the same in emitted Go as in binary64 targets.
revision: 1
---
# Story: The Go writer evaluates float constants as binary64

## Outcome

A `cir/1` expression over float literals means the same in the Go the writer emits as in every
other target: each step rounds to the literal's float type.

## Why

Go evaluates an untyped constant expression exactly. The writer emits `0.1+0.2 == 0.3` and
`-0.0` as written, so Go gives `true` and `+0` where binary64 gives `false` and `-0`
(`adapters/go/cmd/codespec-write-go/float.go:12`, `write.go:999`). `0.0 / 0.0` is written as a
constant division by zero and does not compile. Found by `review-result:
adversary-reader-c-kit-plain-pass-1`, F1. Until this story the C reader refuses an operator whose
operands are all float constants and the negation of a float literal.

## Acceptance

- The Go writer emits a float literal operand in a form Go does not fold exactly (a typed
  conversion of a variable, or a typed constant per step), and `-0.0` as negative zero.
- A kit case or a writer test for `0.1 + 0.2 == 0.3`, `-0.0` and `0.0 / 0.0` prints the same on
  the Go, C and Rust runners.
- The C reader's refusal of constant float operators is removed and its tests expect a read.

## Scope

- Own: `adapters/go/cmd/codespec-write-go/float.go`, `write.go`. Inferred.
