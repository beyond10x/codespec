---
format: aep.planning-md/3
id: review-result:mcp-design-round-1
kind: review-result
status: active
title: Design critic on the MCP epic, round 1
relations:
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
revision: 1
---
needs-revision
story:mcp-rewrite-preview — its last acceptance line asserts that `story:mcp-apply-in-place` refuses an id whose files changed, which cannot be checked until apply lands, and apply depends_on this story, so the line is a reverse dependency; the same refusal is already apply's own acceptance, so drop the refusal clause from the preview and keep only "the preview returns an id" — .engineering/planning/story/mcp-rewrite-preview.md:31, .engineering/planning/story/mcp-apply-in-place.md:35
story:mcp-rewrite-preview — its outcome defines the rule "as `story:cir-patterns` and `story:cir-rewrite` define them", but `story:query-core` supersedes `story:cir-patterns` and docs/query-language.md rejects pattern-by-example as the core and gives rewrites as a node id plus a `cir/1` template, so the body should name only `story:cir-rewrite` as the rule's definition — .engineering/planning/story/mcp-rewrite-preview.md:17, docs/query-language.md:60, docs/query-language.md:283
story:mcp-apply-in-place — it holds two things, a reader-output contract change (a byte-span side file in three adapters, plus `docs/design.md`) and the write tool, and the seam is the span report, which `read_set` in `story:mcp-server-read-tools` must also keep in the read set but that story never says so, so split the span report into its own story that this one and the read tools depend on — .engineering/planning/story/mcp-apply-in-place.md:25, .engineering/planning/story/mcp-apply-in-place.md:43

**What you read**: 6 artifacts in the set (the epic and five stories) plus 5 related ones (`story:cir-patterns`, `story:cir-rewrite`, `story:query-core`, `story:eval-conformance-runner`, `epic:codebase-transformations`), and `story:eval-core` and `story:link-modules` as dependencies. I read them with `aep plan artifact show`, `relations`, `graph` and `validate`, and read `docs/query-language.md` and `docs/evaluator.md` §5.4. I walked about 40 edges touching the set, and walked outside it through `cir-rewrite` to `cir-patterns`, `eval-conformance-runner` to `eval-core`, and the `query-core` supersedes edge. I found no cycle and no serialising chain. `read-tools` is the root, `semantic-tools`, `rewrite-preview` and `apply-in-place` hang off it, `verify-change` hangs off `rewrite-preview`, and `semantic-tools` is a sibling of the preview branch. I traced 5 acceptance reads to a producer in the set, and 4 have a `depends_on` edge. The fifth is the preview's apply-refusal line, which is finding 1. `validate` reported `valid`, with only prose-only review notes on other artifacts.

**What you could not establish**:
- Whether the Go writer can give per-function source. `rewrite_preview` promises "the source the writer gives for the function after", but `story:cir-rewrite` writes whole modules, and I did not find where a per-function text comes from. I did not read the writer code to settle it.
- Whether `crates/eval` exposes the seeded-call generator as a library. `story:eval-conformance-runner` only asserts a task-check step.
- `story:mcp-verify-change` says it takes a "previewed or applied" change but has no edge to `story:mcp-apply-in-place`. Its acceptance reads only preview output, so I did not count it as a finding.
- Out of my lane, scope critic: no story owns the epic's Done When recorded end-to-end session (read, outline, query, preview, apply, re-read, verify).
- Out of my lane, parallel-safety critic: `story:mcp-semantic-tools`, `story:mcp-rewrite-preview` and `story:mcp-verify-change` all own `crates/mcp` after `story:mcp-server-read-tools`.

```findings
- file: .engineering/planning/story/mcp-rewrite-preview.md
  line: 31
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its last acceptance line asserts that story:mcp-apply-in-place refuses an id whose files changed, which cannot be checked until apply lands, and apply depends_on this story, so the line is a reverse dependency; the same refusal is already apply's own acceptance, so drop the refusal clause from the preview and keep only 'the preview returns an id'"
- file: .engineering/planning/story/mcp-rewrite-preview.md
  line: 17
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome defines the rule 'as story:cir-patterns and story:cir-rewrite define them', but story:query-core supersedes story:cir-patterns and docs/query-language.md rejects pattern-by-example as the core and gives rewrites as a node id plus a cir/1 template, so the body should name only story:cir-rewrite as the rule's definition"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 25
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it holds two things, a reader-output contract change (a byte-span side file in three adapters, plus docs/design.md) and the write tool, and the seam is the span report, which read_set in story:mcp-server-read-tools must also keep in the read set but that story never says so, so split the span report into its own story that this one and the read tools depend on"
```
