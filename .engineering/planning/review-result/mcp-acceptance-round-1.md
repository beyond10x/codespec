---
format: aep.planning-md/3
id: review-result:mcp-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic on the MCP epic, round 1
relations:
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
revision: 1
---
needs-revision
story:mcp-apply-in-place — `replace_function` is named in the Outcome, but the acceptance only covers its flag gating and has no bullet that writes a function body given as `cir/1` and reads it back — .engineering/planning/story/mcp-apply-in-place.md:38
story:mcp-apply-in-place — "first: the Go reader; then Rust and C" does not say whether Go alone closes the story, and no check names a Rust or C span — .engineering/planning/story/mcp-apply-in-place.md:25
story:mcp-apply-in-place — the new-import bullet names no fixture or test, and its either/or ("written with it, or the change is refused") is met by either outcome — .engineering/planning/story/mcp-apply-in-place.md:36
story:mcp-apply-in-place — "write only under `--root`" has no test, unlike the three-form refusal test the read story gives for reads — .engineering/planning/story/mcp-apply-in-place.md:38
story:mcp-rewrite-preview — the last bullet asserts that `story:mcp-apply-in-place` refuses a stale id, so this story cannot be closed on its own and both stories claim the same outcome; it should only require that the answer carries an id — .engineering/planning/story/mcp-rewrite-preview.md:31 (duplicates .engineering/planning/story/mcp-apply-in-place.md:35)
story:mcp-rewrite-preview — the Outcome promises "the source the writer gives for the function after", but no acceptance bullet checks that source field, only the "after" documents under strict reading — .engineering/planning/story/mcp-rewrite-preview.md:25
story:mcp-verify-change — the Outcome promises "calls the agent gives" and a `not run` reason for "not exported" and "no calls could be generated", but no bullet checks agent-given calls or those two reasons — .engineering/planning/story/mcp-verify-change.md:29
story:mcp-verify-change — "bounded by a stated step budget per call" names no test showing a call past the budget, and the claim "no timing is measured" has no check — .engineering/planning/story/mcp-verify-change.md:30
story:mcp-server-read-tools — "the server opens no network connection and writes no file" has no test or command that could show it — .engineering/planning/story/mcp-server-read-tools.md:27
story:mcp-semantic-tools — `effects` is promised "for one function or the set", but the acceptance compares only one function with `codespec effects` — .engineering/planning/story/mcp-semantic-tools.md:28

What I read: 5 of 5 drafted stories in full, plus `epic:agent-mcp-server`, `story:cir-patterns`, `story:cir-rewrite`, `story:query-core`, `story:eval-conformance-runner`, via `aep plan artifact show <id>`, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, and `git grep`/`ls` against the tree.

What I could not establish: I did not open `docs/design.md`, `docs/query-language.md` or `docs/evaluator.md` end to end, so I did not check whether `examples/series` and `examples/inventory` hold the cases the acceptances need. Coupling is out of my lane: `story:mcp-semantic-tools` reads `crates/effects` for the `effects` tool but does not depend on `story:link-modules`, which the epic names for linked sets, and that belongs to `plan-critic-design`.

```findings
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 38
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "replace_function is named in the Outcome, but the acceptance only covers its flag gating and has no bullet that writes a function body given as cir/1 and reads it back"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"first: the Go reader; then Rust and C\" does not say whether Go alone closes the story, and no check names a Rust or C span"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 36
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new-import bullet names no fixture or test, and its either/or (written with it, or the change is refused) is met by either outcome"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 38
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "write only under --root has no test, unlike the three-form refusal test the read story gives for reads"
- file: .engineering/planning/story/mcp-rewrite-preview.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the last bullet asserts that story:mcp-apply-in-place refuses a stale id, so this story cannot be closed on its own and both stories claim the same outcome; it should only require that the answer carries an id"
- file: .engineering/planning/story/mcp-rewrite-preview.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the source the writer gives for the function after, but no acceptance bullet checks that source field, only the after documents under strict reading"
- file: .engineering/planning/story/mcp-verify-change.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises calls the agent gives and a not-run reason for not exported and no calls could be generated, but no bullet checks agent-given calls or those two reasons"
- file: .engineering/planning/story/mcp-verify-change.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bounded by a stated step budget per call bullet names no test showing a call past the budget, and the claim that no timing is measured has no check"
- file: .engineering/planning/story/mcp-server-read-tools.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the server opens no network connection and writes no file has no test or command that could show it"
- file: .engineering/planning/story/mcp-semantic-tools.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "effects is promised for one function or the set, but the acceptance compares only one function with codespec effects"
```
