---
format: aep.planning-md/3
id: story:odin-adapters
kind: story
status: implemented
title: Odin reader and writer
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T14:51:14Z", actor: "agent:codespec", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T14:51:14Z", actor: "agent:codespec", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T16:09:39Z", actor: "agent:codespec", revision: 4, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

Read a typed scalar Odin function into `cir/1`, run the passes and write a runnable Odin module. Use Odin's native parser and compiler for the native source.

## Acceptance

- `codespec read --lang odin` is deterministic and preserves or reports unsupported input.
- `codespec write --lang odin` produces code that compiles and runs with the native toolchain.
- Reader and writer conformance reports check the claimed features and the writer boundaries.
- `task language-adapters-check` compares the same function across Elixir, Gleam and Odin.
