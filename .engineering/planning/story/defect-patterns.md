---
format: aep.planning-md/3
id: story:defect-patterns
kind: story
status: draft
title: A checked-in library of defect patterns, run over the measured library
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:cir-patterns
- depends_on: story:library-syscall-map
revision: 1
---
# Story: A checked-in library of defect patterns, run over the measured library

## Outcome

`patterns/go/*.pattern` holds defect patterns written in the pattern language: dropped error,
lock without unlock on every path, map read used without the comma-ok check, a goroutine
capturing a loop variable, a nil check after dereference. `codespec match patterns/go` over
the measured library's linked set writes a findings report.

## Acceptance

- Each pattern has a fixture with one true positive and one near miss that does not match.
- The report on the measured library is recorded with its count per pattern, outside the repository like the library itself.
- Depends on `story:cir-patterns` and `story:library-syscall-map`.
