---
format: aep.planning-md/3
id: epic:whole-repo-to-syscalls
kind: epic
status: active
title: A whole repository mapped down to syscalls
relations:
- informed_by: epic:externs-and-effects
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:52Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:52Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
---
# Epic: A whole repository mapped down to syscalls

## Outcome

Every function of a third-party Go library (the measured library) gets a report of the syscalls it may reach
and the effects they map to, computed from the source of the measured library, its 11 non-standard packages and
the 217 standard library packages it imports (`go list -deps`, Go 1.27.1, linux/amd64), with every
gap listed as `unknown`. On the way, `cir/1` gains the types a real repository uses (§ 5a), and
the linked call graph also answers: which functions are never reached, and what one call does,
drawn as a tree with its branches. Design: `docs/static-analysis.md` § 5, § 5a, § 5b, § 5c.

## Why Now

Timo set the goal "a third-party library can be mapped down to syscalls" (2026-11-08). Measured on the measured library with
the reader of `epic:externs-and-effects`: 36 of 756 functions read with no foreign statement, 971
refusals (§ 5a), so no analysis today reaches below the measured library's own code.

## Out of Scope

- Unhandled branches and security (taint) analyses: `epic:analysis-questions`.
- Reading or writing the new constructs in the Rust, Java and Groovy adapters: they list them
  under `not`; the Rust writer refuses them.
- Platforms other than linux/amd64; cgo; `reflect`-driven calls (reported `unknown`).
- Checking the report against a traced run (§ 6).

## Done When

- `story:library-syscall-map`'s report is checked in and its acceptance holds.
- Every story below is implemented and `task check` passes on the last commit.
