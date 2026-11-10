---
format: aep.planning-md/3
id: review-result:mcp-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic on the MCP epic, round 2
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
story:reader-function-spans — says it "can be worked beside the first three stories" and "touches no file of `crates/mcp`", but it also owns `crates/cli` and `CHANGELOG.md`, which `story:mcp-server-read-tools` owns too (cited, both Scope sections; the two files in `crates/cli` are inferred). The two have no `depends_on` edge. Remedies, without choosing: an ordering edge that records `crates/cli` and `CHANGELOG.md` as its reason, or splitting the surface so they no longer share it. — `.engineering/planning/story/reader-function-spans.md:49-51`, `.engineering/planning/story/mcp-server-read-tools.md:56-57`, `aep plan artifact waves --kind story --status draft` (`collision: story:mcp-server-read-tools story:reader-function-spans crates/cli (inferred)`)

**What you read:** six stories and `epic:agent-mcp-server`, plus the `depends_on` neighbours `story:query-core`, `story:cir-rewrite` and `story:eval-conformance-runner`. Commands: `aep plan artifact show` on each, `aep plan artifact waves --kind story --status draft`, and `git grep` over `crates/cli/src` (`enum Cmd`, `Cmd::Read`) and `CHANGELOG.md`.

Surfaces: 6 of 6 placed from the paths each body's Scope section names, and 0 left unplaced. The store labels every typed scope "inferred". `crates/cli/src/lib.rs` and `main.rs` come from my grep, so they are inferred.

Acceptance reads traced to a producer in the set: 6 of 6, all recorded by `depends_on`, some transitively.
- `apply` reads the span report `reader-function-spans` produces.
- `apply` reads the preview id `mcp-rewrite-preview` creates.
- `semantic-tools` and `rewrite-preview` read the read set `read-tools` creates.
- `verify` reads the previews and the `--write` apply session.

The `crates/mcp` and `CHANGELOG.md` collisions among the five MCP stories form a single `depends_on` chain, and the bodies name it. The `apply` and `spans` overlap on `crates/cli` is ordered by an edge.

**What I could not establish:**
- Whether `reader-function-spans` collides with the "reader stories of the current wave" its Scope names, on the same adapters (`adapters/c-read`, `adapters/rust-read`, `adapters/go/cmd/codespec-read-go`). Those stories are outside this set and have no scope recorded, so I did not judge it. The waves output lists them as unassessed, and the spans body records no edge to them.
- Out of my lane: the epic's sentence that `reader-function-spans` "touches only the reader adapters and `docs/design.md`" disagrees with that story's own scope. That is the epic's text, and it is not part of my verdict.

```findings
- file: .engineering/planning/story/reader-function-spans.md
  line: 51
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body says it can be worked beside the first three MCP stories and touches no file of crates/mcp, but it owns crates/cli and CHANGELOG.md, which story:mcp-server-read-tools also owns (cited in both Scope sections; the two crates/cli files, lib.rs and main.rs, are inferred), and no depends_on edge or admitted overlap exists; add an ordering edge recording the shared files, or split the surface"
```
