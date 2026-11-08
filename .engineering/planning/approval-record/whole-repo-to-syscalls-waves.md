---
format: aep.planning-md/3
id: approval-record:whole-repo-to-syscalls-waves
kind: approval-record
status: draft
title: Timo accepted epic:whole-repo-to-syscalls and asked for all its waves
relations:
- decides: epic:whole-repo-to-syscalls
revision: 1
---
# Approval: accept epic:whole-repo-to-syscalls and run all its waves

Timo's words, 2026-11-08: "/aep:planning stories ... then implement all /wave", and the goal he set:
"a third-party library can be mapped down to syscalls".

- Decided: the epic and its 16 stories move draft → proposed → active (accepted) after the critic
  panel's second round; every wave of the plan derived by `aep plan artifact waves --status active`
  runs without a stop per wave.
- Commits authorised: one commit per unit on `impl/<story>`, its merge into `feat/whole-repo-to-syscalls`
  (the integration branch, stacked on `feat/externs-and-effects`, PR #10), and one store commit per
  wave. No push, no tag, no release: the release stop stays with Timo.
- Not covered: epic:analysis-questions and its two stories stay draft.
