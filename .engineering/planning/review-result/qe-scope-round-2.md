---
format: aep.planning-md/3
id: review-result:qe-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2 (query and evaluator)
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

**What you read:** 9 artifacts. I ran `aep plan artifact show` on the two parent epics and the seven stories, then `aep plan artifact graph` and `aep plan artifact show review-result:qe-scope-round-1`. I also read `docs/query-language.md` and `docs/evaluator.md` in full, plus `story:builtin-defect-checks` and `ls crates/check/tests/fixtures`. I extracted 19 promises, writing the list from the epics and the two docs before reading the stories: query § 9 items 1 to 3 and evaluator § 11 stories 1 to 4, split into their named parts. I traced all 19 to a story. Two of the 19 are named omissions, not gaps: the capslock run and the generated schema page are both in `story:query-core`'s Out of Scope. All three round-1 findings are fixed:
- `story:query-core` now carries `supersedes story:cir-patterns`.
- The cache is claimed, and the schema page is named as left out.
- `story:eval-fault-plans` selects sites from a `--sites` file and defers `--fault-sites` to story 7.

**What you could not establish:**
- Nothing in the scope lane is open. Epic outcome clauses for rewrites, instrumentation, mutation testing and input search belong to the plan items and stories you said are deferred. Existing drafts such as `story:cir-rewrite` and `story:instrumentation` already claim those.
- Out of my lane (design): `story:cir-rewrite` and `story:defect-patterns` still `depends_on story:cir-patterns`, which `story:query-core` now supersedes.
- Out of my lane (design): the new `--sites` flag will sit beside `--fault-sites` once evaluator story 7 lands.
- Out of my lane (design): `story:cir1-defer-receiver-root` depends on four stories of the wave.
- Out of my lane (acceptance): `every_path`, `scope: calls` and the 3-register limit have no acceptance in `story:query-paths-properties`. The `codespec-faults/1` file input has no acceptance in `story:eval-fault-plans`.
- Unease, not a finding: `story:query-core` lets any check wait on § 5.2, § 5.3 or § 8 in a checked-in list. Only `lock-without-unlock` has a later claimant, in `story:cir1-defer-receiver-root`. The design's § 6 writes five of the eight checks in § 5.1 alone, so I expect the list to hold only that one.

```findings
[]
```
