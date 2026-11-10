---
format: aep.planning-md/3
id: review-result:mcp-scope-round-1
kind: review-result
status: active
title: Scope critic on the MCP epic, round 1
relations:
- reviews: epic:agent-mcp-server
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
revision: 1
---
needs-revision
epic:agent-mcp-server — Done When promises "a recorded session of a scripted MCP client against a checked-in example shows: read, outline, one cq query, one rewrite previewed, applied, re-read equal, and verified in the evaluator", and no story claims that end-to-end recorded session (each story's acceptance is per-tool tests; the example is not named in the chain); `story:mcp-verify-change`, last in the chain, would most naturally take it — .engineering/planning/epic/agent-mcp-server.md:54
story:mcp-apply-in-place — the epic promises "Every change is shown as a diff before it is written" and "checked after", but `replace_function` writes an agent-supplied body with no preview step, no diff and no strict-reading check in the outcome or any acceptance — .engineering/planning/epic/agent-mcp-server.md:20
story:mcp-apply-in-place — the epic's write-refusal promise covers only languages with no writer, yet the span report is claimed for "first: the Go reader; then Rust and C", acceptance checks Go only, and nothing says what `apply` does for Rust and C files until their spans land — .engineering/planning/epic/agent-mcp-server.md:50
story:mcp-rewrite-preview — the epic says every change is "shown as a diff", but the outcome returns `cir/1` before and after and only the writer's source for the "after", so no diff of source text is claimed despite the title "a diff out" — .engineering/planning/epic/agent-mcp-server.md:20
story:mcp-apply-in-place — the `--write` startup flag that removes `apply` and `replace_function` from `tools/list` traces to no sentence in the epic, whose Outcome has the connected agent changing code and whose Open questions do not name it — .engineering/planning/story/mcp-apply-in-place.md:38

What I read: 6 artifacts (the parent plus 5 stories), via `aep plan artifact show` on each, `aep plan artifact graph`, and `show` on `story:cir-rewrite`, `story:cir-patterns`, `story:eval-conformance-runner` and `story:query-core`.

Promises: I extracted 14 from the parent and traced 11 to an item. The three that are not fully traced are the recorded session (a gap), the diff-before-write promise (narrowed in `story:mcp-rewrite-preview` and unclaimed for `replace_function`), and write behaviour for Rust and C (narrowed).

What I could not establish:
- Out of my lane, did not set the verdict: `story:mcp-rewrite-preview` (line 17) names `story:cir-patterns` as the pattern definition, but `story:query-core` supersedes it. That is a design-critic matter.
- Out of my lane: the acceptance for `story:mcp-semantic-tools` is checkable as written, so I raise nothing there.
- The second open question (read set in memory or on disk) is handled correctly. `story:mcp-server-read-tools` holds the set in process memory and the epic leaves the question undecided. I count it as neither a gap nor a reach.

```findings
- file: .engineering/planning/epic/agent-mcp-server.md
  line: 54
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Done When promises a recorded session of a scripted MCP client against a checked-in example (read, outline, one cq query, one rewrite previewed, applied, re-read equal, verified) and no story claims that end-to-end session; story:mcp-verify-change would most naturally take it"
- file: .engineering/planning/epic/agent-mcp-server.md
  line: 20
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic promises every change is shown as a diff before it is written and checked after, but story:mcp-apply-in-place's replace_function writes an agent-supplied body with no preview, no diff and no strict-reading check in the outcome or any acceptance"
- file: .engineering/planning/epic/agent-mcp-server.md
  line: 50
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic refuses writes only for languages with no writer, but story:mcp-apply-in-place claims spans for Go first then Rust and C with acceptance for Go only and says nothing of what apply does for Rust and C files meanwhile, so the promise is narrowed without a recorded decision"
- file: .engineering/planning/epic/agent-mcp-server.md
  line: 20
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic says every change is shown as a diff, but story:mcp-rewrite-preview returns cir/1 before and after and only the writer's source for the after, so no source diff is claimed despite its title"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 38
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the --write startup flag that removes apply and replace_function from tools/list traces to no sentence in the epic's Outcome, Out of Scope or Open questions"
```
