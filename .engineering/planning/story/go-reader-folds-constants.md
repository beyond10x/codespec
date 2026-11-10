---
format: aep.planning-md/3
id: story:go-reader-folds-constants
kind: story
status: draft
title: The Go reader folds constant expressions
summary: A Go constant expression is read as the literal Go computed, not operator by operator.
revision: 1
---
# Story: The Go reader folds constant expressions

## Outcome

A Go constant expression reaches `cir/1` as the literal Go computed, so every consumer that
evaluates `cir/1` with typed machine arithmetic gets Go's answer.

## Why

Go evaluates an untyped constant expression exactly and converts once. The Go reader lifts
`1 << 62 >> 1`-style and `0.1 + 0.2` expressions operator by operator, so a consumer that wraps
at 64 bits or rounds at each step prints something else than `go run`
(`review-result:adversary-eval-core-pass-1`, F1: `ok 0` against `ok 2305843009213693952`, and
`0.30000000000000004` against `0.3`). `cir/1` operators are typed machine arithmetic; the
evaluator and the writers of other targets are right to compute them that way. The counterpart
on the writing side is `story:go-writer-float-constants`.

## Acceptance

- The Go reader writes the value `go/types` computed for a constant expression as one literal of
  the expression's type, where that value fits the type; an expression with a non-constant operand
  is unchanged.
- The evaluator test `adv_constant_expressions_are_exact_as_go_folds_them` runs without its
  ignore marker and passes.
- Every checked-in `case.cir.json` is regenerated; every writer and reader report keeps its
  `pass` count.

## Scope

- Own: `adapters/go/cmd/codespec-read-go`. Inferred.
