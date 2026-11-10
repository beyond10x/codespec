---
format: aep.planning-md/3
id: review-result:mcp-scope-round-2
kind: review-result
status: active
title: Scope critic on the MCP epic, round 2
relations:
- reviews: epic:agent-mcp-server
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
- reviews: story:reader-function-spans
revision: 1
---
needs-revision
story:mcp-apply-in-place — the acceptance requires that after `apply` "the file imports" `strings`, which writes bytes outside every changed function's span and contradicts the story's own outcome ("every other byte of the file is kept") and the parent's promise of byte-identical code outside the changed functions — .engineering/planning/story/mcp-apply-in-place.md:45 (parent: .engineering/planning/epic/agent-mcp-server.md:23 "It is written only to the bytes of the functions it changed" and :54 "Code outside the changed functions stays byte-identical")

What I read: 7 artifacts, via `aep plan artifact show` on the parent and the six stories, plus `aep plan artifact graph`, `aep plan artifact list`, `show` on `story:cir-rewrite` and `review-result:mcp-scope-round-1`, and `grep` for "import" in the apply story and the round-1 reviews.

Promises: I extracted 21 from the parent and traced 20 to an item, listed below.
- Stdio server: `story:mcp-server-read-tools`.
- cir/1 reads with refusals and positions, bounded answers, the in-memory read set, and read-only when not started with `--write`: `story:mcp-server-read-tools`.
- cq queries, callers and callees, and effects: `story:mcp-semantic-tools`.
- Rewrite and replaced-body changes, previewed and strict-read, shown as a source diff: `story:mcp-rewrite-preview`.
- Matches that hold a refusal are never touched, and languages with no writer are refused: `story:mcp-rewrite-preview`.
- Only a previewed change can be written, writes go to function spans only, the changed file is re-read and compared, and `--write` gates the writing tool: `story:mcp-apply-in-place`.
- A language with no spans is refused by name: `story:mcp-apply-in-place`.
- A differential run in the evaluator for changes that claim to keep behaviour, and the recorded session: `story:mcp-verify-change`.
- Function spans beside `cir/1` for Go, Rust and C: `story:reader-function-spans`.

The one I could not trace to a covering sentence is "Code outside the changed functions stays byte-identical". `story:mcp-apply-in-place` claims it in its outcome and then contradicts it in line 45, hence the finding. No uncovered gap and no two items claiming one outcome.

What I could not establish:
- Out of my lane, did not set the verdict: how `apply` would add an import without writing outside the spans is a design question for the design critic. The fix may be to drop the import bullet, or to refuse a change that needs a new import.
- `--root` and the symbolic-link and network-crate tests in `story:mcp-server-read-tools` are not named in the parent. I read them as constraints on promised behaviour, not as reach, so I raised nothing.

```findings
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 45
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires that after apply the file imports strings, which writes bytes outside every changed function's span and contradicts the story's outcome ('every other byte of the file is kept') and the epic's promise that code outside the changed functions stays byte-identical"
```
