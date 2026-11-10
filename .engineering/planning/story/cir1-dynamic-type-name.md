---
format: aep.planning-md/3
id: story:cir1-dynamic-type-name
kind: story
status: draft
title: 'cir/1: the name of a value''s dynamic type'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: cir/1 has the name of a value's dynamic type

## Outcome

`cir/1` has an intrinsic that gives the name of a value's dynamic type as a string, with a stated
form of that name per source language, so the Java reader reads `x.getClass().getName()` and
`getSimpleName()`.

## Why

Measured by `story:cir1-raise-out-of-entry` on the Quarkus quickstart: 2 of its 8 remaining
refusals are `exception.getClass().getName()`, one in each exception mapper, refused as reflection.

## Acceptance

- `cir/1` documents the intrinsic and what the string is for a value read from Java and from Go
  (`fmt.Sprintf("%T", x)` is the Go source form the Go reader reads).
- The Java reader reads `getClass().getName()` and `getClass().getSimpleName()` on a value whose
  static type is a class or interface; any other use of the `Class` value stays refused.
- The evaluator answers the intrinsic for the values it holds; a writer that cannot give the same
  string refuses it by name.
- The quickstart reads with 2 refusals fewer.

## Scope

- Own: `crates/model` (intrinsics), `adapters/java/src/codespec/read/Reader.java`,
  `adapters/go/cmd/codespec-read-go`, `crates/eval/src/builtins.rs`, the writers' intrinsic tables.
  Inferred.
