---
format: aep.planning-md/3
id: story:query-lookup-skips-wildcard-tuples
kind: story
status: draft
title: A cq lookup does not visit every wildcard tuple
summary: Store::candidates appends every tuple with a wildcard value to each lookup; recursive rules pay for it per derived tuple.
revision: 2
---
# Story: a cq lookup does not visit every wildcard tuple

## Outcome

A recursive `cq` rule over a few thousand facts finishes in a number of join steps that grows with
the tuples that match a lookup, not with the tuples of the relation that hold a wildcard value.

## Why

`Store::candidates` (`crates/query/src/engine.rs:181` to `193`) indexes the tuples without a
wildcard value by the bound columns and then appends `self.tops`, every tuple that holds one, to
the result of every lookup. A relation with many such tuples makes each lookup linear in them,
and a recursive rule repeats the lookup for every derived tuple.

Two reports, neither reproduced in this repository:

- a set of five recursive rules over 234 documents: four did not finish in 600 s;
- on 2026-11-11, a two-rule transitive closure with one negation over 1269 call edges of one
  document took 46 minutes of CPU on a debug build of `5480cc12`, where the built-in `reaches`
  answered in seconds.

Whether the second report has the same cause is not known; the story starts by finding out.

## Acceptance

- A test builds a store with a relation of N tuples, M of them holding a wildcard value, runs a
  transitive-closure rule and counts the tuples `candidates` returns (a counter in the engine,
  test-only): the count does not grow with M for lookups whose key no wildcard tuple can match.
- The answers of every existing query test are unchanged, wildcard matches included.
- The cause of the second report is named in this story, with the rule that shows it written
  against `adapters/groovy/testdata/holds` or a made-up document.
- No timing is asserted or recorded; the measure is the count.
- `task check` exits 0.
