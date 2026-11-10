---
format: aep.planning-md/3
id: review-result:mcp-design-round-2
kind: review-result
status: active
title: Design critic on the MCP epic, round 2
relations:
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
- reviews: story:reader-function-spans
revision: 1
---
needs-revision
story:mcp-apply-in-place — its outcome keeps "every other byte of the file" and replaces only function spans, yet an acceptance line requires the file to gain an import (`strings`) that lies outside every function span, and `story:reader-function-spans` reports only per-function spans, so the body must say where the import edit comes from (an import-block span added to that story with an edge to it, or a stated import rule in apply) — .engineering/planning/story/mcp-apply-in-place.md:27, .engineering/planning/story/mcp-apply-in-place.md:45, .engineering/planning/story/reader-function-spans.md:28

**What you read**: 11 artifacts (the epic, the five stories, `story:cir-rewrite`, `story:cir-patterns`, `story:query-core`, `story:eval-conformance-runner`, `epic:codebase-transformations`) plus `story:link-modules`, `story:eval-core` and `review-result:mcp-design-round-1`. Commands: `aep plan artifact show`, `relations`, `graph`, `validate` (`valid`), and `docs/design.md`, `docs/query-language.md`, `docs/evaluator.md`. I walked about 35 edges and went outside the set via `cir-rewrite`, `cir-patterns`, `query-core`, `eval-conformance-runner`, `eval-core` and the epic's own `depends_on` edges. I found no cycle. I traced 8 acceptance reads to a producer in the set, and 8 have an edge. The ninth read is the import edit in apply, which is the finding above.

Round 1 against this set: the reverse-dependency line is gone from the preview. The preview now names only `story:cir-rewrite`. The span report is its own story with an edge from apply. The chain `read-tools`, `semantic-tools`, `rewrite-preview`, `apply-in-place`, `verify-change` carries its reason in `story:mcp-rewrite-preview` (shared tool table), so I did not report it as a serialising chain.

**What you could not establish**:
- Whether `story:cir-rewrite` gives the preview a library entry point and writers beyond Go. Its body (revision 1, `.engineering/planning/story/cir-rewrite.md`) still defines a pattern-by-example rule, depends on the superseded `story:cir-patterns`, and writes through the Go writer only. The preview and apply acceptances ask for Go, Rust and C. That is a stale body outside this set, so I did not make it a finding here.
- Whether `crates/eval` exposes the seeded-call generator as a library, which `story:mcp-verify-change` needs. `story:eval-conformance-runner` only asserts a `task check` step.
- Out of my lane, parallel safety: `story:reader-function-spans` lists `crates/cli` and `CHANGELOG.md` in its scope, while the epic body says it touches only the adapters and `docs/design.md`. It also says it runs after the current wave's reader stories, which edit the same adapters, and no edge records that.

```findings
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 45
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome keeps every other byte of the file and replaces only function spans, yet an acceptance line requires the file to gain an import (strings) that lies outside every function span, and story:reader-function-spans reports only per-function spans, so the body must say where the import edit comes from (an import-block span added to that story with an edge to it, or a stated import rule in apply)"
```
