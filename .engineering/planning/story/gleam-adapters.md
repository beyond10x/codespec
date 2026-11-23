---
format: aep.planning-md/3
id: story:gleam-adapters
kind: story
status: implemented
title: Gleam reader and writer
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T14:51:14Z", actor: "agent:codespec", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T14:51:14Z", actor: "agent:codespec", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T15:27:06Z", actor: "agent:codespec", revision: 4, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

Read a typed scalar Gleam function into `cir/1`, run the passes and write a runnable Gleam module. Use the Gleam Glance parser and the Erlang backend for the native source.

## Acceptance

- `codespec read --lang gleam` is deterministic and preserves or reports unsupported input.
- `codespec write --lang gleam` produces code that compiles and runs with the native toolchain.
- Reader and writer conformance reports check the claimed features and the writer boundaries.
- `task language-adapters-check` compares the same function across Elixir, Gleam and Odin.
