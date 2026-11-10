---
format: aep.planning-md/3
id: story:mcp-verify-change
kind: story
status: draft
title: 'MCP verify: a change that claims to keep behaviour is run before and after'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:mcp-rewrite-preview
- depends_on: story:eval-conformance-runner
- depends_on: story:mcp-apply-in-place
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/mcp
revision: 3
---
# Story: MCP verify: a change that claims to keep behaviour is run before and after

## Outcome

The tool `verify` takes the id of a preview and runs each changed exported function, before and
after, in the evaluator on the same calls: calls the agent gives, and seeded calls as the
differential step of `story:eval-conformance-runner` generates them. It answers per function
`equal`, `differs` with the first differing call and both outputs, or `not run` with the reason
(the evaluator stopped, the step budget ran out, the function is not exported, no calls could be
generated for its types). This story also owns the recorded end-to-end session of
`epic:agent-mcp-server`.

## Acceptance

- For the rename rewrite on a fixture where `G` equals `F`: every changed function answers
  `equal` over 200 seeded calls.
- For a fixture rewrite that changes behaviour (`x + 1` to `x + 2`): the function answers
  `differs` with a call and both printed results.
- With calls given by the agent (a `codespec-calls/1` document) and no seeds, the answer covers
  those calls only: a test gives one call on which before and after differ and one on which they
  are equal, and each is answered for itself.
- A function the evaluator stops on answers `not run` with the stop's reason; an unexported
  changed function answers `not run: not exported`; a function with a parameter type no call can
  be generated for answers `not run` naming the type; a function whose loop exceeds the stated
  step budget per call answers `not run: step budget`. One test each; none answers `equal`.
- The answer's JSON Schema has no duration or time field, by a test over `tools/list`.
- The recorded session: a test drives the server (started with `--write` on a copy of
  `examples/series`) through read, outline, one cq query, one rewrite previewed, applied, re-read
  equal and verified, and compares the transcript with a checked-in file under
  `crates/mcp/tests/session/`.

## Scope

- Own: `crates/mcp/src/tools/verify.rs`, the tool table in `crates/mcp/src/lib.rs`,
  `crates/mcp/tests/session/`, `CHANGELOG.md`. Runs after `story:mcp-apply-in-place` (the session
  applies a change). Reads `crates/eval` through its library interface. Depends on
  `story:eval-conformance-runner` for seeded calls. Inferred.
