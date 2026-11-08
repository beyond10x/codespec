---
format: aep.planning-md/3
id: story:instrumentation
kind: story
status: draft
title: 'Instrument a codebase: logging, timing and counters at entries and effect sites'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:cir-rewrite
revision: 1
---
# Story: Instrument a codebase: logging, timing and counters at entries and effect sites

## Outcome

Built-in rewrite rules insert, per function or per effect site (an extern call or syscall path
from the effects report), a log line, a timer or a counter, and write the instrumented program
back as Go. Running it writes `codespec-measure-run/1`: calls, time and effect sites hit, per
function.

## Acceptance

- The envcheck example instrumented at every `env` effect site logs each `os.Getenv` with its
  position; the run's counts match the calls of the harness trace.
- Depends on `story:cir-rewrite` and the effects report of `epic:whole-repo-to-syscalls`.
