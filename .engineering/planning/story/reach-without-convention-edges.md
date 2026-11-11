---
format: aep.planning-md/3
id: story:reach-without-convention-edges
kind: story
status: draft
title: reach and reaches can leave out convention edges
summary: An option follows only the call edges a type checker resolved; the default still follows call refs that carry by.
relations:
- depends_on: story:reader-groovy-call-lists-for-refused-code
revision: 2
---
# Story: reach and reaches can leave out convention edges

## Outcome

`codespec reach`, and the `reaches` relation of `cq`, can be asked to follow only the call edges a
type checker resolved, leaving out every call ref that carries `by`. The default is unchanged:
convention edges are followed.

## Why

Since the `by` field, a reader may name a target by a convention (`grails-gorm`,
`grails-service-field`). A reviewer needs both answers: what is reached when the conventions hold,
and what is reached on checked edges alone. Today the second answer needs a hand-written
transitive closure in `cq` that excludes `call_by` sites (`adapters/groovy/README.md` shows it).
Reported by another session on 2026-11-11 for one Grails 2.5.6 application (not named): with
convention edges 4598 caller and callee pairs, without them 1885; the built-in `reaches` answered
in seconds and the hand-written closure over 1269 edges took 46 minutes of CPU on a debug build.
Not reproduced here.

## Acceptance

- `codespec reach` takes an option that leaves out call refs with `by`; on
  `adapters/groovy/testdata/holds` the reached set with the option equals the set the README's
  hand-written query gives, and without it the set of today.
- `cq` has a relation or a variant of `reaches` with the same meaning, listed by
  `codespec query --schema` and in `docs/query-language.md`.
- A document without any `by` gives the same answer with and without the option.
- `task check` exits 0.

## Depends on

`story:reader-groovy-call-lists-for-refused-code`, which introduces `by` and `call_by`.
