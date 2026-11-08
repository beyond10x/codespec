---
format: aep.planning-md/3
id: review-result:whole-repo-design-r1
kind: review-result
status: active
title: Design critic, round 1, epic:whole-repo-to-syscalls
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
needs-revision

Findings, one per line:

- story:cir1-arrays-floats — its outcome says `uintptr` and `unsafe.Pointer` become extern types "of the barrier", but no body creates that. The design says they "stay at the barrier" (docs/static-analysis.md:161), and `story:foreign-calls` needs a statement using `unsafe.Pointer` to stay foreign, with no edge ordering the two. Drop the clause, or move it into `story:syscall-barrier` and add an edge from `story:foreign-calls` to whichever story makes the change. — .engineering/planning/story/cir1-arrays-floats.md:59, .engineering/planning/story/foreign-calls.md:48, docs/static-analysis.md:161
- story:foreign-calls — its `CallRef` variants "interface method, call of a function value, a function used as a value" are halves of constructs that `story:cir1-interfaces` and `story:cir1-func-values` define (`Expr::IfaceCall`, `Expr::CallValue`, `Expr::FuncRef`). It depends only on `story:cir1-extern-types`. Add `depends_on` edges to both stories, or move those three variants into the stories that create the expressions. — .engineering/planning/story/foreign-calls.md:40, .engineering/planning/story/cir1-interfaces.md:64, .engineering/planning/story/cir1-func-values.md:64
- story:call-trace-view — it builds on the linked call graph, but `story:reachability` is the one that moves that graph "to one function both use", and no `depends_on` edge runs from this story to `story:reachability`. This story's acceptance names the test file only as `crates/effects/tests/trace.rs (or the crate the graph lives in)`. Add `depends_on story:reachability`. — .engineering/planning/story/call-trace-view.md:41, .engineering/planning/story/reachability.md:35
- story:call-trace-view — its outcome has the site draw "one entry point of the measured library". That needs the 234 linked documents and the effects of the measured library that `story:library-syscall-map` produces in the library example directory, and the documents are not checked in. No edge records this. Add `depends_on story:library-syscall-map`, or drop the library from the outcome. — .engineering/planning/story/call-trace-view.md:30-31, .engineering/planning/story/library-syscall-map.md:36,45

What I read: 17 artifacts (the epic and its 16 stories). I ran `aep plan artifact list --format json`, `show` on all 17, `relations`, `graph` and `validate`. Validate reported valid.

What I could not establish:
- Edges: I walked about 45 edges on the full graph, including those that leave the set (`epic:externs-and-effects`, `epic:analysis-questions`, `story:cir1-externs`). I found no cycle, and no chain that serialises the set (the longest path is five items).
- Acceptance reads: I traced about 19 reads to a producer in the set. 16 are recorded by an edge, directly or transitively. 3 are not, and they are the 3 findings above. The 3 are the `unsafe.Pointer` fixture, the shared graph, and the documents of the measured library.
- Not a defect: the 8 `cir1-*` stories each cut across reader, writer, model and conformance, so the set is not a horizontal slice. Their sibling edges (`go-reader-whole-package`, `cir1-extern-types`) all record the reads I traced.
- Out of my lane, not counted in the verdict:
  - Eleven stories list the same ~21 `cir1-*` surface files (`schemas/cir-1.schema.json`, `crates/model/src/lib.rs`, `conformance/kit.json` and others) with no ordering edge between sibling `cir1-*` stories. That is for `plan-critic-parallel-safety`.
  - `story:call-trace-view` promises output on the measured library that its acceptance does not check. That is for `plan-critic-acceptance`.
  - `story:syscall-barrier`'s `os.ReadFile` acceptance depends on opaque-function fallback for any stdlib construct not yet expressible. I could not judge whether that is enough.

```findings
- file: .engineering/planning/story/cir1-arrays-floats.md
  line: 59
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the outcome makes uintptr and unsafe.Pointer extern types of the barrier, which no body implements, the design keeps them at the barrier, and story:foreign-calls needs an unsafe.Pointer statement to stay foreign with no edge ordering the two; drop the clause or move it to story:syscall-barrier and add the edge"
- file: .engineering/planning/story/foreign-calls.md
  line: 40
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its CallRef variants for interface method, function-value call and function-as-value are halves of Expr::IfaceCall, Expr::CallValue and Expr::FuncRef that story:cir1-interfaces and story:cir1-func-values define, and it depends only on story:cir1-extern-types; add depends_on edges to both or move the variants into those stories"
- file: .engineering/planning/story/call-trace-view.md
  line: 41
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it builds on the linked call graph that story:reachability moves to one shared function (reachability.md:35), and no depends_on edge runs from it to story:reachability; add depends_on story:reachability"
- file: .engineering/planning/story/call-trace-view.md
  line: 31
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome draws one entry point of the measured library, which needs the documents and effects of the measured library that story:library-syscall-map produces in the library example directory (documents not checked in, library-syscall-map.md:45), and no edge records it; add depends_on story:library-syscall-map or drop the library from the outcome"
```
