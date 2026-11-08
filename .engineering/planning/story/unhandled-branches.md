---
format: aep.planning-md/3
id: story:unhandled-branches
kind: story
status: draft
title: Report branches a program does not handle
relations:
- decomposes: epic:analysis-questions
revision: 1
---
# Story: Report branches a program does not handle

## Outcome

`codespec check unhandled <documents…>` lists, with positions: a `switch` over a value of a
type with a closed set (a newtype with declared constants, a type switch over an interface's known
implementations in the set) missing a member and with no `default`; and every call of a fallible
function whose error result is assigned to `_` or dropped (`crates/passes` `fallible`).

## Acceptance

- A fixture with one incomplete switch and one dropped error yields exactly those two entries.
- The result on the measured library is recorded with its count, outside the repository like the library itself.
