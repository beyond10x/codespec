---
format: aep.planning-md/3
id: story:reach-at-positions
kind: story
status: draft
title: codespec reach answers for a list of file:line positions
relations:
- decomposes: epic:sound-reach
- depends_on: story:reader-function-spans
- depends_on: story:reader-indirect-call-sites
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: website/docs/guides/
revision: 6
---
# Story: codespec reach answers for a list of file:line positions

## Outcome

`codespec reach --at <file>` takes a JSON list of records, each with a `pos` (`file:line`) and
any other fields, and writes the same list with `reach` added to each record. Other fields pass
through byte for byte, in the input's order. `reach` is a list holding, in this order:

- one entry per function holding the line (two when two definitions share it), each with
  `function`, `status`, for `reached` its `distance` and `nearest_entry`, and `via_unknown`:
  whether the status is `unknown`;
- one entry `{"answer": "not read", "reason": …}` per refusal of the document set at this
  position, with the reason from the module's refusal list;
- when the list would be empty, the single entry `{"answer": "no function"}`.

Which function holds a line: with the span file of `story:reader-function-spans`
(`--spans <file>`), the function whose span holds it. Without it, the function with the greatest
`pos` line at or before the line in that file, and each such entry carries `approximate: true`;
a line before the first function of its file, or in a file no document names, has no holding
function.

## Acceptance

In `crates/cli/tests/reach.rs`, each against a checked-in expected output:

- Four records (a line in a reached function, in an `unknown` one, in a file no document
  names, and before the first function of a file the set does name) get a `reached` entry, an
  `unknown` entry with `via_unknown: true`, and twice the single `no function` entry; the
  input's other fields and order are kept.
- A position that a refusal of the set names, inside a function: `reach` has two entries, the
  function's status and then `not read` with that refusal's reason.
- A line holding two function definitions gives two function entries.
- A line of a function's closing brace, without `--spans`, is answered for that function with
  `approximate: true`; with `--spans` on a Go example it has no `approximate`.

## Scope

- Own: `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/tests/reach.rs`,
  `crates/effects/src/lib.rs`, `website/docs/guides` (one section), `CHANGELOG.md`. Inferred;
  recorded as typed scope.
- Depends on `story:reader-indirect-call-sites`, the story before it on the reach pass in
  `crates/effects` and `crates/cli`, and on `story:reader-function-spans` for `--spans`. It
  shares no file with the three other reader stories and can be worked beside them.
