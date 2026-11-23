---
format: aep.planning-md/3
id: story:review-effects-summaries-and-roots
kind: story
status: implemented
title: 'Effects: listed methods take summaries, summaries match their language, every Go init is a root'
summary: Fix maintainer-review findings Q3, Q4 and Q9 in crates/effects
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/effects/src/lib.rs
- confidence: cited
  path: crates/effects/src/set.rs
- confidence: cited
  path: crates/effects/tests/effects.rs
- confidence: cited
  path: crates/effects/tests/reach.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:44:29Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:44:30Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T03:44:37Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: review-effects-summaries-and-roots

## Outcome

A user of `codespec effects`, `trace` and `reach` gets the summarised effects of a method call a
foreign statement lists, never has a Go effect cleared by a summary written for another language,
and sees every function a Go package's second or later `init` calls as reached.

## Context

Findings Q3, Q4 and Q9 of the maintainer review of 0.10.0
(`.engineering/reviews/2026-11-23-maintainer-review.md`), under
`epic:maintainer-review-2026-11-23`.

## Findings

- Q3, listed external method calls lose known effects: a `method` call ref naming a module outside
  the set now goes through the same summary lookup as a typed `extern_method` call
  (`<type>.<method>` of its path). Test:
  `crates/effects/tests/effects.rs::a_listed_external_method_call_takes_the_summary_of_the_typed_call`.
- Q4, `reach` followed only the first Go `init`: every `init` function node of a Go module is a
  root; the key map still holds one lookup for the name. Test:
  `crates/effects/tests/reach.rs::every_go_init_function_is_an_entry`.
- Q9, a summary of another language cleared a Go effect: summaries are looked up by the reader
  language of the calling module, then path and name. Test:
  `crates/effects/tests/effects.rs::a_summary_of_another_language_does_not_apply`.

## Acceptance

- The three tests above fail on 0.10.0 and pass with the change.
- `cargo test -p b10x-codespec-effects --locked` and `task examples-effects` are green.

## Done When

The pull request from `review/effects` is merged into `main` with the three tests green in CI.

## Out of Scope

- The effects report still lists one entry per function key, so a second `init`'s own effects
  are not a separate report entry.
- Mapping between related languages (a Groovy module reading a Java summary) is not added.

## Ambiguities

None.

## Open Questions

None.
