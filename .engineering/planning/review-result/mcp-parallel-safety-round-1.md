---
format: aep.planning-md/3
id: review-result:mcp-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic on the MCP epic, round 1
relations:
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
revision: 1
---
needs-revision
story:mcp-semantic-tools — it lands on `crates/mcp` (cited, its Scope line), the same crate that `story:mcp-rewrite-preview`, `story:mcp-apply-in-place` and `story:mcp-verify-change` each name, and no `depends_on` path or sentence orders it against any of them (both depend on `story:mcp-server-read-tools`, so they are siblings in one wave). Either record the shared files as the reason for an ordering edge, or split `crates/mcp` per tool so the items no longer share a surface. `crates/mcp` does not exist yet, so every item adds to a crate that `story:mcp-server-read-tools` creates. — .engineering/planning/story/mcp-semantic-tools.md:35, .engineering/planning/story/mcp-rewrite-preview.md:36, .engineering/planning/story/mcp-apply-in-place.md:43, .engineering/planning/story/mcp-verify-change.md:34
story:mcp-verify-change — it and `story:mcp-apply-in-place` both land on `crates/mcp` (cited, both Scope lines) and both depend on `story:mcp-rewrite-preview` with no edge or sentence between them. Either add an ordering edge that records the shared crate as its reason, or split the surface. — .engineering/planning/story/mcp-verify-change.md:34, .engineering/planning/story/mcp-apply-in-place.md:43
story:mcp-rewrite-preview — its acceptance line 5 says `story:mcp-apply-in-place` applies by the preview's id and refuses a stale id, which only `apply` can show, yet `apply` depends on this story. The reader is a producer's own acceptance and the edge runs the wrong way. Move that behaviour into `story:mcp-apply-in-place`'s acceptance (it already has the stale-id line), or merge the two. — .engineering/planning/story/mcp-rewrite-preview.md:31, .engineering/planning/story/mcp-apply-in-place.md:35

**What you read**
- Six artifacts: the five stories and `epic:agent-mcp-server`.
- Commands: `aep plan artifact show` on each, plus `aep plan artifact graph`, `relations` and `waves`.
- I also ran `aep plan artifact show` on `story:query-core`, `story:cir-rewrite`, `story:cir-patterns`, `story:eval-conformance-runner` and `story:link-modules`.
- I listed `crates/` and `adapters/` to check that the named paths exist.

**What you could not establish**
- Surfaces: 5 items placed cited, 0 inferred, 0 unplaced. The bodies say "Inferred" in their Scope lines, and none has typed scope. "Cited" here means the body names `crates/mcp` and similar paths, at whole-crate granularity only. I could not tell which files inside `crates/mcp` each item touches.
- Reads traced to a producer in the set: 5, of which 4 are recorded by `depends_on`:
  - `semantic-tools` reads from `read-tools`.
  - `rewrite-preview` reads from `read-tools`.
  - `apply-in-place` reads from `rewrite-preview`.
  - `verify-change` reads from `rewrite-preview`.
  - The unrecorded one is `rewrite-preview` reading `apply-in-place`, which is the finding above.
- `story:mcp-verify-change` says it takes "a previewed or applied change", but its acceptance only uses previewed fixtures. I did not call that a finding, since no acceptance line reads state `apply` writes.
- `story:mcp-server-read-tools` is the only item that names `crates/cli`, `CHANGELOG.md` and `AGENTS.md`. The other four will also need changelog entries, but their bodies cite none, so I did not guess.
- Out of my lane:
  - The Scope sections are prose, not typed scope, so `aep plan artifact waves` lists all five as unassessed.
  - Whether five stories on one crate is the right split is for the design critic.
- Items outside the set: collisions with other stories on `crates/cli` and the reader adapters were not assessed.

```findings
- file: .engineering/planning/story/mcp-semantic-tools.md
  line: 35
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it lands on crates/mcp (cited), the same crate story:mcp-rewrite-preview, story:mcp-apply-in-place and story:mcp-verify-change each name, with no depends_on path or sentence ordering it against any of them; add an ordering edge recording the shared files as its reason, or split crates/mcp per tool (crates/mcp does not exist yet; story:mcp-server-read-tools creates it)"
- file: .engineering/planning/story/mcp-verify-change.md
  line: 34
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and story:mcp-apply-in-place both land on crates/mcp (cited) and both depend on story:mcp-rewrite-preview with no edge or sentence between them; add an ordering edge recording the shared crate as its reason, or split the surface"
- file: .engineering/planning/story/mcp-rewrite-preview.md
  line: 31
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance line 5 reads story:mcp-apply-in-place's behaviour (apply by id, refuse a stale id) while apply depends_on this story, so the edge runs from producer to reader and the line cannot be checked here; move that behaviour into story:mcp-apply-in-place's acceptance or merge the two"
```
