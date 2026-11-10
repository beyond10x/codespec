---
format: aep.planning-md/3
id: story:readers-bound-untrusted-input
kind: story
status: implemented
title: Readers bound what an untrusted source can cost
summary: Nesting depth and input size have limits in every reader; past a limit is a refusal, not a crash.
relations:
- decomposes: epic:adapters-untrusted-input
- depends_on: story:reader-rust-kit
- depends_on: story:reader-c-kit-conventions
- depends_on: story:reader-java-quarkus-refusals
scope:
- confidence: inferred
  path: adapters/c-read
- confidence: inferred
  path: adapters/rust-read
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-10T17:52:03Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: Readers bound what an untrusted source can cost

## Outcome

A source file cannot overflow a reader's stack or exhaust memory: nesting depth and input size
have stated limits, and a file past a limit is refused with exit 1 and a message.

## Acceptance

- `codespec-read-rust`: parsing and lifting run on a thread with a stated stack size and a stated
  nesting-depth limit; a source nested past the limit is refused, not a crash. A test generates
  such a source (deeply nested parentheses and blocks) and asserts exit 1 and the message.
- `codespec-read-c`: the size of the JSON AST `clang` prints is bounded by a stated limit read as
  a stream, and the reader's own passes over it are linear in its size; a test with a generated
  source that made the earlier reader's memory grow quadratically asserts a refusal or a read
  within a stated node budget. No timing is measured: the test counts nodes or bytes.
- `codespec-read-go` and the JVM readers: the report states for each whether its parser already
  bounds depth (and where), with one generated deep-nesting test each that ends in exit 0 or a
  refusal, never a crash.
- The limits are documented in each reader's crate doc or README and in `docs/conformance.md`.
- Every reader report under `website/data/conformance/` keeps its `pass` count.

## Scope

- Own: `adapters/rust-read`, `adapters/c-read`, `adapters/go/cmd/codespec-read-go`,
  `adapters/java`, `adapters/jvm` (limits only). Inferred. Runs after the reader kit stories of
  `epic:reader-coverage`, which edit the same crates.
