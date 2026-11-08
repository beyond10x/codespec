---
format: aep.planning-md/3
id: story:taint-flows
kind: story
status: draft
title: Report flows from an untrusted source to a dangerous sink
relations:
- decomposes: epic:analysis-questions
revision: 1
---
# Story: Report flows from an untrusted source to a dangerous sink

## Outcome

`codespec check taint <documents…>` follows values from sources (results of externs or syscalls
with effect `net`, `stdin`, `env`) through assignments, calls and returns in the linked set to
sinks (externs with effect `proc` or `fs.write`, declared in a sink table), and reports each
flow as the path of positions.

## Acceptance

- A fixture where a value read from a connection reaches `os/exec.Command` reports one flow with
  its positions; the same fixture with the value replaced by a constant reports none.
- The result on the measured library is recorded outside the repository, like the library itself.
