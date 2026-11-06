---
format: aep.planning-md/3
id: story:rust-writer-error-propagation
kind: story
status: draft
title: The Rust writer propagates an error from one fallible call into another
relations:
- informed_by: story:java-example-ledger
revision: 1
---
# Story: The Rust writer propagates an error from one fallible call into another

## Outcome

`codespec-write-rust` writes a fallible function that calls another fallible function and returns
its error (`v, err := f(); if err != nil { return …, err }`) as Rust with `?` or an explicit
`match`, instead of refusing it.

## Context

Found while building `examples/ledger` (story java-example-ledger, wave 2, 2026-11-06). A Java call
that throws reads into this form, so any Java method calling a throwing method hits it. The ledger
was reshaped so that `Account.post` does not throw. This limitation already existed in the writer.

## Acceptance

- A writer test: a document in which a fallible function calls another and returns its error is
  written as Rust that compiles. A test run shows the error reaching the caller unchanged.
- The orders, greeter, inventory, series and ledger outputs are unchanged.

## Scope

- `adapters/rust/src/`.
