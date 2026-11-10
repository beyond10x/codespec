---
format: aep.planning-md/3
id: story:reader-indirect-sites-rust
kind: story
status: draft
title: The Rust reader states the calls it cannot resolve as indirect sites
relations:
- decomposes: epic:sound-reach
- depends_on: story:reader-indirect-call-sites
- depends_on: story:reader-rust-kit
- depends_on: story:reader-function-spans
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/rust-read/
- confidence: inferred
  path: examples/reach-rust/
revision: 4
---
# Story: the Rust reader states the calls it cannot resolve as indirect sites

## Outcome

The Rust reader gives `indirect` (`story:reader-indirect-call-sites`) for a method call on a
`dyn Trait` value it keeps foreign (`trait-object`, with `name` and `through` the trait), a call
of an `fn` pointer (`fn-pointer`), and a call of a closure stored in a field or variable that it
does not read (`closure`, `through` the field where there is one). A call it lists in `calls`
gets no entry.

## Acceptance

- `adapters/rust-read/tests`: three checked-in Rust files, one per kind above, each with its
  expected `indirect` entries.
- Ground truth: `examples/reach-rust`, checked in with its entry list and the expected report
  `expected.json`; `task reach-examples` runs it with a line printed at each function's start,
  requires every printed function to be `reached` or `unknown`, and compares the report with
  `expected.json`. It calls through a boxed trait object, an `fn` pointer and a stored closure.
  Three types implement the trait: one made in reached code, one made only in a function
  nothing calls, one made only in a function called only through the stored closure. In
  `expected.json` a function nothing could call and no site fits is `unreachable`, the three
  types' trait methods are `unknown` with the site's position, and a named function has a
  distance written there.
- `git diff --exit-code website/data/conformance/reader-rust.json` after `task docs-generate`.

## Scope

- Own: `adapters/rust-read`, `examples/reach-rust`, `Taskfile.yml`, `CHANGELOG.md`. Inferred;
  recorded as typed scope.
- Depends on `story:reader-indirect-call-sites` (the field and the matching) and on
  `story:reader-rust-kit` and `story:reader-function-spans` (the same adapter).
