---
format: aep.planning-md/3
id: story:library-syscall-map
kind: story
status: implemented
title: A third-party library is mapped down to syscalls
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:syscall-barrier
- depends_on: story:dispatch-over-set
- depends_on: story:cir1-embedded-fields
- depends_on: story:cir1-module-vars
- depends_on: story:cir1-variadic
- depends_on: story:cir1-arrays-floats
- depends_on: story:cir1-generics
scope:
- confidence: cited
  path: docs/static-analysis.md
- confidence: cited
  path: scripts/read-repo.sh
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:58Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:58Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T06:06:04Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: a third-party library is mapped down to syscalls

## Outcome

A third-party Go library (the measured library of `docs/static-analysis.md` § 5a) and its 228
imported packages (Go 1.27.1, linux/amd64) are read with `scripts/read-repo.sh --deps`, linked,
and reported by `codespec effects` and `codespec reach`: for every function of the library's 6
packages, its effects, its syscalls and each `unknown` with a reason. The measured example and its
report are kept outside this repository; `docs/static-analysis.md` § 5d records the numbers that
do not identify the library: functions with no `unknown`, the syscalls of the entry points.

## Context

- Entry points: the library's client and server entry points, and every handler reached through
  its registration of handlers as function values (`story:dispatch-over-set`).
- The documents of the 234 packages, the report and the script that wrote them are not in this
  repository. The source was fetched with `go mod download` at a pinned version.

## Acceptance

The acceptance below was checked on the measured example, which is kept outside this repository;
it is not checkable from this repository.

- The task that maps the example exits 0 and its output is byte-identical across two runs.
- The client entry point's syscalls include `sendto` or `write`, and `connect` or `socket`; the
  server entry point's include `bind` or `listen`.
- The example's README states the count of the library's functions with an empty `unknown`, from
  the report.
- An offline check of the example's report passes (no network).
