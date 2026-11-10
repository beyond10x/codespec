---
format: aep.planning-md/3
id: story:c-reader-plain-shift-count
kind: story
status: draft
title: The C reader bounds the count of a plain shift
relations:
- decomposes: epic:cir-evaluator
revision: 1
---
# Story: the C reader bounds the count of a plain shift

## Outcome

The C reader reads a plain `x << n` or `x >> n` only when the count is known to be below the
width of `x`, and refuses it otherwise; the kit's C shift cases are written so that they read.

## Why

Found by the second adversary pass on `story:reader-c-kit-conventions`
(`review-result:adversary-reader-c-kit-conventions-pass-2`, H3), in the reader before that unit:
a plain `x << n` with `n` of 64 or more reads as Go's shift, which gives 0, while the C on x86
gives `x << (n & 63)`. A refusal costs four kit claims, because
`conformance/cases/{op,assign}-{shl,shr}/c/case.c` use a plain shift by a variable count.

## Acceptance

- `adapters/c-read/tests/adversary3.rs::a_plain_shift_by_a_count_the_reader_cannot_bound_is_refused`
  runs without its `#[ignore]` and passes.
- The four kit cases read (their C uses a constant or masked count, or `cs_shl`/`cs_shr`), and
  `codespec conform --lang c --role reader` keeps its claimed count.

## Scope

- Own: `adapters/c-read/src/body.rs` (the shift arm), the four `case.c` files, the generated
  reports. Inferred.
