---
format: aep.planning-md/3
id: review-result:qe-design-round-2
kind: review-result
status: active
title: Design critic, round 2 (query and evaluator)
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
approve

**What you read:** 7 stories via `aep plan artifact show`, plus `relations`, `graph` and `validate` (valid). I also read `review-result:qe-design-round-1`, `docs/query-language.md` §§ 4, 5, 7, 8 and 9, `docs/evaluator.md` § 11, and `story:builtin-defect-checks`.

**What I walked:**
- I followed 16 edges: 8 `depends_on`, 7 `decomposes` and 1 `supersedes`. They include the ones that leave the set (`epic:codebase-transformations`, `epic:cir-evaluator`, `story:cir-patterns`). I found no cycle.
- I traced 8 acceptance reads to a producer in the set, and an edge records every one, directly or through an edge that already runs to it. Examples:
  - `cir1-defer-receiver-root` reads the property from `query-paths-properties`.
  - It reads the `query-core` list through `query-paths-properties`, which depends on `query-core`.
  - It reads the conformance report from `eval-conformance-runner`.
  - It reads the `orders` trace that `eval-fault-plans` consumes from `eval-handlers-traces`.
- The round 1 blocker is closed. `query-core` now sends `lock-without-unlock` to a checked-in waiting list, and the equivalence acceptance sits in `cir1-defer-receiver-root`, which is last and depends on both stories it needs.
- Slot E is a linear chain. Each `after` edge has the shared `crates/eval` crate written beside it in Scope, so I did not report it.

**What I could not establish:**
- Out of my lane, for scope or acceptance: `cir1-defer-receiver-root` adds the `defer` statement and `recv_var`, but its acceptance does not say it refreshes the `--schema` expected file that `query-core` checks in.
- Out of my lane, for scope: after `query-paths-properties` lands, nothing migrates the checks the `query-core` list says are waiting on § 5.2 or § 5.3.
- Not a defect today: the node-id scheme of § 4.1 is built separately in `crates/query` and `crates/eval`, in parallel slots, with no shared crate or agreement test. `docs/evaluator.md` E5 and line 6 do tie them. Only `story:eval-fault-plans` fixes a literal id (`orders.Service.Pay#3:1`), and nothing pins the two implementations to each other before `docs/evaluator.md` § 11 story 7.

```findings
[]
```
