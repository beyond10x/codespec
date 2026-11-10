---
format: aep.planning-md/3
id: epic:adapters-untrusted-input
kind: epic
status: active
title: Adapters hold against untrusted input
summary: No name from a cir/1 document reaches generated output unvalidated; readers bound nesting and size.
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 3}
---
# Epic: Adapters hold against untrusted input

## Outcome

A `cir/1` document or a source file from an untrusted party cannot make a writer emit code or
build configuration its author did not put in a body, and cannot crash or exhaust a reader.

## Why Now

A review of this repository at `4204699` on 2026-11-10 reproduced five defects: two where a name
from a `cir/1` document is written unescaped into generated output, one writer panic, and two
readers without a bound on nesting or size. The repository is private until they are fixed.

## Done When

- `story:writers-validate-names` and `story:readers-bound-untrusted-input` are implemented.

## Out of Scope

- Sandboxing the runners: a runner compiles and runs the code it is given, by design.
