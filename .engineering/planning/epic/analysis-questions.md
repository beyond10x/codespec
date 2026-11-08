---
format: aep.planning-md/3
id: epic:analysis-questions
kind: epic
status: draft
title: 'Analyses on the linked call graph: unhandled branches and security'
relations:
- depends_on: epic:whole-repo-to-syscalls
revision: 1
---
# Epic: Analyses on the linked call graph: unhandled branches and security

## Outcome

Two passes over the linked set of `epic:whole-repo-to-syscalls`: one reports branches a program
does not handle, one reports flows from an untrusted source to a dangerous sink. Design:
`docs/static-analysis.md` § 5c, rows "later".

## Why Now

Filed from Timo's list of questions codespec should answer (2026-11-08). Not started: both need
the linked call graph, which `epic:whole-repo-to-syscalls` builds.

## Out of Scope

- Anything the linked set does not hold yet.

## Done When

- Both stories are implemented, each with a test on a fixture and its result on the measured library recorded.
