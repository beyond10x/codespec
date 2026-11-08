---
format: aep.planning-md/3
id: review-result:whole-repo-design-r2
kind: review-result
status: active
title: Design critic, round 2, epic:whole-repo-to-syscalls
relations:
- reviews: epic:whole-repo-to-syscalls
- reviews: story:go-reader-whole-package
- reviews: story:cir1-extern-types
- reviews: story:cir1-interfaces
- reviews: story:cir1-func-values
- reviews: story:cir1-embedded-fields
- reviews: story:cir1-module-vars
- reviews: story:cir1-variadic
- reviews: story:cir1-arrays-floats
- reviews: story:cir1-generics
- reviews: story:foreign-calls
- reviews: story:link-modules
- reviews: story:dispatch-over-set
- reviews: story:syscall-barrier
- reviews: story:library-syscall-map
- reviews: story:reachability
- reviews: story:call-trace-view
revision: 1
---
approve

What I read: 17 artifacts, the epic and its 16 stories. I ran `aep plan artifact list --format json`, `show review-result:whole-repo-design-r1`, `relations`, `graph` and `validate` (valid). I read every story body at its current revision. I walked about 55 edges on the full graph, including those that leave the set (`epic:externs-and-effects`, `epic:analysis-questions`, `story:cir1-externs`), and found no cycle. The longest path is eight items: `go-reader-whole-package`, `cir1-extern-types`, `cir1-interfaces`, `foreign-calls`, `link-modules`, `syscall-barrier`, `library-syscall-map`, `call-trace-view`. Each edge on it has a read behind it, and the five `cir1-*` siblings run in parallel, so this is not a chain that serialises the set. I traced about 22 acceptance reads to a producer in the set, and an edge records every one, directly or transitively.

Round 1 findings, all four closed:
- **`story:cir1-arrays-floats`:** the clause that made `uintptr` and `unsafe.Pointer` extern types is gone. The outcome (line 59) now says they stay outside `cir/1` and the statement stays `foreign`. Its one acceptance read of that behaviour (line 83) uses the existing foreign statement, not `foreign-calls` output.
- **`story:foreign-calls`:** it now has `depends_on` edges to `story:cir1-interfaces` and `story:cir1-func-values`, and the context (lines 123-124) says why.
- **`story:call-trace-view`:** it now has `depends_on story:reachability`.
- **`story:call-trace-view`:** it now has `depends_on story:library-syscall-map`.

What I could not establish:
- `story:cir1-arrays-floats` line 59 names `story:foreign-calls` in prose with no edge. I did not make it a finding, because no acceptance line reads anything that story produces.
- Out of my lane, and it does not set the verdict. The eleven stories that share the ~21 `cir1-*` surface files, such as `schemas/cir-1.schema.json` and `crates/model/src/lib.rs`, still have no ordering edge between the sibling `cir1-*` stories. That is for `plan-critic-parallel-safety`.
- Out of my lane. Whether `story:syscall-barrier`'s `os.ReadFile` fixture reads cleanly through the standard library is for `plan-critic-acceptance`.
- I ran one redirect into a file outside the repository by mistake and deleted it straight away. It was never tracked, and the store was not touched.

```findings
[]
```
