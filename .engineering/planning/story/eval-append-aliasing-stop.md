---
format: aep.planning-md/3
id: story:eval-append-aliasing-stop
kind: story
status: draft
title: The evaluator stops where append aliasing depends on Go's escape analysis
summary: No silent answer where two appends may share an array Go could have put on the stack.
revision: 1
---
# Story: The evaluator stops where append aliasing depends on Go's escape analysis

## Outcome

A program whose output depends on whether two appends share a backing array that Go may have
put on the stack stops with a reason instead of printing an answer Go might not print.

## Why

The evaluator follows the heap growth rule of Go's runtime. Go's compiler can give a slice that
does not escape a stack buffer of another capacity, so
`a = append(nil, 1); b := append(a, 2); c := append(a, 3); return b[1]` can print 2 in the
evaluator and 3 under Go. Stated as a limit in the crate doc of `crates/eval`. Found while
answering `review-result:adversary-eval-core-pass-1`, F8.

## Acceptance

- The evaluator tracks which slices may share an array after an append that did not have to
  grow, and stops with a reason when a write through one is read through another and the sharing
  depended on a capacity the stack rule could change.
- The kit keeps `134 printed, 1 stopped as listed`.

## Scope

- Own: `crates/eval`. Inferred.
