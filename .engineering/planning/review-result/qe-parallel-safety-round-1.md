---
format: aep.planning-md/3
id: review-result:qe-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (query and evaluator)
relations:
- reviews: story:query-core
- reviews: story:query-paths-properties
- reviews: story:cir1-defer-receiver-root
- reviews: story:eval-core
- reviews: story:eval-conformance-runner
- reviews: story:eval-handlers-traces
- reviews: story:eval-fault-plans
revision: 1
---
needs-revision
story:query-core — both this and story:eval-core land on `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, the single `members = [...]` line of `Cargo.toml`, `CHANGELOG.md`, `website/docs/reference/{cli,crates}.md`, `crates/docs/src/status.rs` and `AGENTS.md` (cited, both Scope sections, which the bodies themselves mark Inferred), they are the first stories of slots Q and E so they run together, and neither body says so; remedies are an ordering edge recording the shared files as its reason, or splitting those surfaces so the two no longer share them — .engineering/planning/story/query-core.md:48, .engineering/planning/story/eval-core.md:33
story:query-paths-properties — its `CHANGELOG.md` and "generated site reference" entries overlap `CHANGELOG.md` and generated site reference in story:eval-conformance-runner (via `crates/docs`, `AGENTS.md`), story:eval-handlers-traces and story:eval-fault-plans, which run in slot E beside it, and the body does not say so (cited, Scope sections; whether this story actually changes the generated reference is inferred); remedies are an ordering edge naming the shared files, or splitting them — .engineering/planning/story/query-paths-properties.md:31, .engineering/planning/story/eval-handlers-traces.md:28
story:cir1-defer-receiver-root — its Scope puts `crates/eval` "where they match on statement kinds", and story:eval-conformance-runner, story:eval-handlers-traces and story:eval-fault-plans all land on `crates/eval` (cited, all bodies) and can run while this story is third in slot Q, but its only edge into the evaluator chain is `depends_on story:eval-core` and no body names the overlap; remedies are an ordering edge from this story to the last evaluator story that edits statement dispatch, or splitting the `crates/eval` statement handling out of this story — .engineering/planning/story/cir1-defer-receiver-root.md:39, .engineering/planning/story/eval-conformance-runner.md:29
story:cir1-defer-receiver-root — a new defer case in `conformance/` and `schemas/cir-1.features.json` regenerates `conformance/kit.json`, `conformance/cases/*`, the reader corpora and `website/data/conformance/{reader,writer}-<lang>.json` (cited, AGENTS.md generated-files table), which is the surface story:eval-conformance-runner writes with its drift-checked `--lang cir` report, and its acceptance (`conforms: true`, "like the six existing ones") reads the kit this story changes; there is no `depends_on` edge either way, so the edge between the two stories, or merging them, is needed — .engineering/planning/story/cir1-defer-receiver-root.md:39, .engineering/planning/story/eval-conformance-runner.md:29, AGENTS.md:128-133
story:cir1-defer-receiver-root — its Scope says "every generated file `AGENTS.md` lists for a model change", which names no files, so its footprint cannot be compared to any concurrent story and it is unassessed, not safe; the body should list the generated files it rewrites — .engineering/planning/story/cir1-defer-receiver-root.md:39

What I read: 7 stories, plus `AGENTS.md` and the tree for existence checks. Commands: `aep plan artifact show` on all seven, `aep plan artifact graph`, `aep plan artifact relations`, `aep plan artifact waves`, and `ls`/`grep` on `Cargo.toml`, `AGENTS.md` and `crates/*`. Items placed: 7 cited, 0 inferred beyond the bodies' own Inferred tag, 1 partly unplaceable (cir1's generated-file list). Acceptance reads traced to a producer in the set: 3 (query-paths on query-core, eval-conformance-runner on eval-core, and eval-conformance-runner on cir1's kit change), of which 2 are recorded by a `depends_on` edge.

What I could not establish:
- Every Scope is typed Inferred, so each surface is the drafter's, not a recorded scope entry.
- Whether query-paths-properties really changes the generated site reference.
- Whether the evaluator's own `foreign`/`defer` handling is part of eval-core's acceptance. That bears on whether the kit case cir1 adds breaks eval-core's "every kit case" step, and it is out of my lane (acceptance and design critics).
- Ordering inside each slot is already carried by `depends_on` edges; I report no collision there.

```findings
- file: .engineering/planning/story/query-core.md
  line: 48
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story and story:eval-core both land on crates/cli/src/lib.rs, main.rs, the Cargo.toml members line, CHANGELOG.md, generated website/docs/reference/{cli,crates}.md, crates/docs/src/status.rs and AGENTS.md, run first in slots Q and E together, and neither body says so; add an ordering edge recording the shared files or split the surface (cited, both Scope sections marked Inferred)"
- file: .engineering/planning/story/query-paths-properties.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its CHANGELOG.md and generated site reference overlap those of story:eval-conformance-runner, story:eval-handlers-traces and story:eval-fault-plans, which run in slot E beside it, and the body does not say so; add an ordering edge or split the surface (cited Scope sections; whether this story changes the generated reference is inferred)"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 39
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its Scope edits crates/eval where it matches on statement kinds, which story:eval-conformance-runner, story:eval-handlers-traces and story:eval-fault-plans also land on, and its only edge into the evaluator chain is depends_on story:eval-core; add an ordering edge recording crates/eval as the reason or split the evaluator statement handling out (cited)"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 39
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the new defer case in conformance/ and cir-1.features.json regenerates the kit, reader corpora and website/data/conformance reports that story:eval-conformance-runner writes and whose acceptance (conforms: true for --lang cir) reads, and no depends_on edge runs between them; add the edge or merge the two (cited, AGENTS.md generated-files table)"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 39
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Scope entry 'every generated file AGENTS.md lists for a model change' names no files, so the story cannot be compared with concurrent stories and is unassessed; list the generated files it rewrites"
```
