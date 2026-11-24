---
format: aep.planning-md/3
id: decision-blocker:cir1-test-only-count-field
kind: decision-blocker
status: open
title: Does cir/1 take an additive field for items a reader left out as test-only?
summary: story:rust-reader-count-test-only-items needs the count in the cir/1 module; no existing field holds it without changing its meaning.
relations:
- blocks: story:rust-reader-count-test-only-items
revision: 1
---
# Decision: does cir/1 take an additive field for items a reader left out as test-only?

## Question

`story:rust-reader-count-test-only-items` needs `codespec measure` to say how many functions and
items a reader left out because a normal build has them only under `cfg(test)`. `measure` reads
only the `cir/1` module, and no field of it holds the count without changing that field's
meaning (`doc`, `facts`, `refusals`, `opaque`; the story's `## Model change` says why each).

## Options

- A (default, recommended): an additive `Module` field, for example `left_out: [{pos, what,
  reason}]`, omitted when empty, so every existing document stays valid and byte-equal.
- B: no model change; the reader prints the count on stderr only. `measure` cannot see it, so the
  story's Done When is not met and the story is rewritten or archived.
- C: no count; the story is archived with that reason.

## Cleared when

The owner picks an option; this artifact moves to `cleared` with the pick written here.
