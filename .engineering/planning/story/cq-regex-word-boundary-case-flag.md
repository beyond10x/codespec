---
format: aep.planning-md/3
id: story:cq-regex-word-boundary-case-flag
kind: story
status: implemented
title: cq regular expressions have word boundaries and a case-insensitive flag
summary: \b, \B and a leading (?i) in the ~ operator
relations:
- decomposes: epic:codebase-transformations
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T14:18:44Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-11-13T14:18:44Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-11-13T14:18:44Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cq regular expressions have word boundaries and a case-insensitive flag

## Outcome

The `~` operator of cq accepts `\b` and `\B` (a word boundary and its negation, a word character
being a letter, digit or `_`) and a leading `(?i)` flag that matches letters regardless of case,
with the same linear-time guarantee as the rest of its regex dialect.

## Why

Rules written today spell a word boundary as `(^|[^A-Za-z0-9_])` and a case-insensitive word as
`[Tt]oken`; both forms are longer, and the first consumes a character, so two adjacent matches
interfere.

## Acceptance

- `X ~ "\\bunsafe\\b"` matches `unsafe {` and `x unsafe`, not `unsafely`; `\B` the opposite.
- `X ~ "(?i)token"` matches `Token`, `TOKEN` and `token`; `(?i)` anywhere other than the start of
  the pattern is refused with a message naming it.
- The regex tests in `crates/query` cover each, and `docs/query-language.md` lists the dialect.
- Every existing query gives the same rows.

## Scope

- Own: `crates/query/src/regex.rs`, `docs/query-language.md`. Inferred.
