---
format: aep.planning-md/3
id: review-result:qe-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1 (query and evaluator)
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
story:query-core — the body says it "takes the place of `story:cir-patterns`" but records only `informed_by`, so `story:cir-patterns` (`codespec match`, "finds code by shape") stays a live draft claiming the same epic outcome, and both would be marked done; add a `supersedes` relation or otherwise retire `story:cir-patterns` — .engineering/planning/story/query-core.md:24 (with .engineering/planning/story/cir-patterns.md:15 and .engineering/planning/epic/codebase-transformations.md:16)
story:query-core — `docs/query-language.md` § 4 promises the fact-base cache ("caches it next to the documents (`<dir>/.cq/<digest>.facts` …)") and a generated schema page ("the documentation site gets a generated schema page"), and neither the outcome, acceptance, scope nor out-of-scope mentions either, so they are silently dropped; claim them or name them as left out — docs/query-language.md:80-83
story:eval-fault-plans — the outcome says "`--each` runs once per selected site", but the only site selector in § 6.2 (`--fault-sites <cq query>`, plan story 7) is outside this wave and the epic's exclusion, and the body neither says how sites are selected nor records that `--fault-sites` and `--max-nth` are deferred, so the story either quietly claims part of story 7 or promises an `--each` with nothing to select — .engineering/planning/story/eval-fault-plans.md:16 (with docs/evaluator.md:175-176 and .engineering/planning/epic/cir-evaluator.md Out of Scope)

**What you read:** 9 artifacts (2 parent epics and 7 stories) with `aep plan artifact show` on each, plus `aep plan artifact graph` and `relations`. I also read `docs/query-language.md` and `docs/evaluator.md` in full, and `story:cir-patterns` for the overlap.

**What you could not establish:**
- Promises extracted: 13 from the two epics and the plan items in scope (the 5 epic outcome clauses plus plan items 1 to 3 of the query design and stories 1 to 4 of the evaluator design, one per promise). I traced 13 to an item, with the three findings above as the exceptions in quality, not in count.
- Not set against the epic's Outcome: the pattern-language, rewrite, instrumentation and mutation clauses. You said plan items 4 to 6 and evaluator stories 5 to 8 are deliberately left for a later wave, and existing drafts (`story:cir-rewrite`, `story:instrumentation`, `story:mutation-testing`, `story:defect-patterns`) already claim them. The evaluator's "search for inputs" clause (story 5) is the same.
- Out of my lane (acceptance): `every_path`, `scope: calls` and the 3-register limit of § 5.3 have no acceptance in `story:query-paths-properties`. `story:eval-handlers-traces` has no acceptance for per-site handler replacement or user-named stubs (E3, E4). `story:eval-fault-plans` has no acceptance for `--each` or for the five `do` kinds.
- Out of my lane (design): `story:cir1-defer-receiver-root` depends on `story:query-paths-properties`, although the query design's property example needs `defer` as an input. The reversed order may be intended.
- Out of my lane (design): `story:eval-core` has an acceptance line that lets unrunnable kit cases be listed rather than failed, which sits beside E1 "every kit case".

```findings
- file: .engineering/planning/story/query-core.md
  line: 24
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body says it takes the place of story:cir-patterns but records only informed_by, so story:cir-patterns stays a live draft claiming the same outcome (find code by shape) and both would be marked done; add a supersedes relation or otherwise retire story:cir-patterns"
- file: docs/query-language.md
  line: 80
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "docs/query-language.md § 4 promises the fact-base cache (.cq/<digest>.facts) and a generated schema page, and story:query-core's outcome, acceptance, scope and out-of-scope mention neither, so they are silently dropped; claim them or name them as left out"
- file: .engineering/planning/story/eval-fault-plans.md
  line: 16
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome says --each runs once per selected site, but the only site selector (--fault-sites, plan story 7) is out of this wave and the body neither says how sites are selected nor records that --fault-sites and --max-nth are deferred, so the story either claims part of story 7 or promises an --each with nothing to select"
```
