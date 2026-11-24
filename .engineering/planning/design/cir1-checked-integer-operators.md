---
format: aep.planning-md/3
id: design:cir1-checked-integer-operators
kind: design
status: draft
title: 'Checked integer operators in cir/1: the design note'
summary: How cir/1 states a Rust integer operator (+ - * neg / % << >>) whose overflow depends on the build, written and adversary-reviewed before story:rust-reader-integer-overflow is implemented.
relations:
- designs: story:rust-reader-integer-overflow
- decomposes: epic:async-network-service-translation
revision: 1
---
# Design: Checked integer operators in cir/1

## Outcome

A design note says how `cir/1` states a Rust integer operator whose result on overflow depends on
the build, additively, before `story:rust-reader-integer-overflow` is implemented. It works out
that story's decision D1 = A (a checked integer operator in `cir/1`). The note is the `## Note`
section of this artifact.

## Done When

- The note exists as the `## Note` section of
  `.engineering/planning/design/cir1-checked-integer-operators.md` and states:
  - the operator family: `+`, `-`, `*`, unary `-`, `/`, `%`, `<<` and `>>`, with their compound
    assignments;
  - the three behaviours and which node carries each: dependent on `overflow-checks` (`+ - *` and
    unary `-` wrap without it and panic with it; a shift by a count at or past the operand's
    width W masks the count without it and panics with it); a panic in every build (`/` and `%`
    of the type's minimum by -1); explicit (`wrapping_*`, `checked_*`, `saturating_*` and
    `overflowing_*` calls);
  - how the reader takes `overflow-checks` from the Cargo profile it is told (the manifest's
    `[profile.*]` setting, else the profile's default), or refuses when it cannot tell, and
    where the module records the configuration it was read for;
  - that the profile input and the module's recorded configuration are the ones
    `story:rust-reader-cfg-features` uses for `debug_assertions`;
  - that every evaluator and writer that does not know the operator refuses it, none ignores it;
  - the surfaces the implementation will touch, by path (at least `crates/model/src/lib.rs`,
    `schemas/cir-1.schema.json`, `schemas/cir-1.features.json`, `crates/eval`,
    `adapters/rust-read/src/`, the Go writer in `adapters/go`, `adapters/rust/src/write.rs` and
    the C writer in `adapters/c`).
- An adversary review of the note is recorded: a `review-result` that `reviews` this artifact,
  with a verdict (`approve` or `needs-revision`). The note moves to `approved` only on an
  `approve`.

## Scope

- `.engineering/planning/design/cir1-checked-integer-operators.md`. No code.
