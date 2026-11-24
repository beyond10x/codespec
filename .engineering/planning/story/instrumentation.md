---
format: aep.planning-md/3
id: story:instrumentation
kind: story
status: draft
title: 'Instrument a codebase: logging, timing and counters at entries and effect sites'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:edit-replace-template
- depends_on: story:edit-imports
revision: 2
---
# Story: Instrument a codebase: logging, timing and counters at entries and effect sites

## Outcome

Built-in cq `edit` programs insert, per function entry or per effect site (an extern call or
syscall path from the effects report), a log line, a timer or a counter (`insert_before` of
`story:edit-replace-template`, the imports they need through `story:edit-imports`). `crates/edit`
applies them to a cache copy of the module, keeping the original files' formatting in the copy.
Running the instrumented copy writes `codespec-measure-run/1`: calls, time and effect sites hit,
per function.

## Acceptance

- The envcheck example instrumented at every `env` effect site logs each `os.Getenv` with its
  position; the run's counts match the calls of the harness trace.
- The instrumented copy differs from the original only by the inserted lines and imports (its diff
  against the original has no deleted line).
- Depends on `story:edit-replace-template`, `story:edit-imports` and the effects report of
  `epic:whole-repo-to-syscalls`.

Restated on `codespec edit` on 2026-11-24 (`epic:code-edit-language`); before, the program was
written back through the Go writer (`story:cir-rewrite`, superseded).
