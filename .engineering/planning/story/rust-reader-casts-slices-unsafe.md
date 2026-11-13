---
format: aep.planning-md/3
id: story:rust-reader-casts-slices-unsafe
kind: story
status: draft
title: The Rust reader reads integer casts, slice ranges and unsafe items
summary: as casts as conv, &[T] and range indexing, an unsafe record
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-whole-crate
revision: 3
---
# Story: The Rust reader reads integer casts, slice ranges and unsafe items

## Outcome

Three constructs the Rust reader refuses today are read or recorded:

1. `e as T` between integer types is a `conv` to `T` of `e`, whose type is the source type, so a
   query compares the widths.
2. A `&[T]` parameter is a `slice`, and a range index `s[a..b]`, `s[..b]`, `s[a..]` of a slice,
   a `Vec` or a `String` is read in the form `cir/1` has for it, or stays a refusal naming the
   form when `cir/1` has none.
3. An `unsafe fn`, an `unsafe impl` and an `unsafe { … }` block are recorded so a query can name
   each, without changing what any existing field means.

## Acceptance

- A test per construct: the cast's `conv` with its two types; the slice parameter and the range
  index; the `unsafe` record for a function, an impl and a block.
- A cast whose meaning differs from Go's conversion (a float to an integer saturates in Rust) is
  refused, not read as `conv`.
- Any `cir/1` addition is additive and regenerates the schema; checked-in examples read unchanged.

## Scope

- Own: `adapters/rust-read`; `crates/model` only for an additive record. Inferred.

## Findings

Not started. Found while planning, 2026-11-13:

- An integer cast has no `cir/1` form today: `conv` is "a conversion to the named type `name`"
  (`crates/model/src/lib.rs`, feature `expr.conv`), and no intrinsic converts between integer
  widths. Reading `e as u16` needs an addition every writer and the evaluator implement (a
  conversion to a basic integer type that truncates or extends as Go's and Rust's do), with its
  own conformance case; that is a model change, not a reader change alone.
- A range index has no `cir/1` form either: `index` takes one index. A slice expression with a low
  and a high bound is likewise a model addition with writer and evaluator support.
- An `unsafe` record has no field to go on today; an additive one (on a function, an opaque
  function and a `foreign` statement) is the smallest form.
