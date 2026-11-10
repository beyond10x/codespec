---
format: aep.planning-md/3
id: story:kit-string-ordering
kind: story
status: draft
title: The kit orders strings of different lengths
relations:
- decomposes: epic:cir-evaluator
revision: 1
---
# Story: the kit orders strings of different lengths

## Outcome

The conformance kit's cases for `op.lt`, `op.le`, `op.gt` and `op.ge` compare strings as well as
integers, with operands of different lengths and a shared prefix, so an adapter that orders
strings by length first fails them.

## Why

Found by the adversary pass on `story:eval-conformance-runner`
(`review-result:adversary-eval-conformance-runner-pass-1`, F4): an evaluator changed to compare
string lengths first passes every kit case and `task eval-diff`, while `op.lt` and `op.gt` are
claimed. The kit tests `<` on integers only.

## Acceptance

- `conformance/` has string operands in the four ordering cases (`"b" < "ab"` is false,
  `"ab" < "b"` is true, `"a" < "ab"` is true); every adapter that claims the features passes,
  or its claim is withdrawn with the reason.
- `crates/eval/tests/adversary3.rs::adv3_mut_strings_order_byte_by_byte_not_by_length` stays.

## Scope

- Own: `conformance/` (four cases and their expected files), the generated reports. Inferred.
