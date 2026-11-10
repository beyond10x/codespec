---
format: aep.planning-md/3
id: review-result:reach-scope-round-1
kind: review-result
status: active
title: Scope critic on the reach epic, round 1
relations:
- reviews: epic:sound-reach
- reviews: story:reach-three-valued
- reviews: story:reader-indirect-call-sites
- reviews: story:reach-entry-file
- reviews: story:reach-at-positions
- reviews: story:reach-registered-candidates
- reviews: story:reach-framework-entries
revision: 1
---
needs-revision

- epic:sound-reach — "a checked-in example per reader language (Go, Rust, C, Java/Groovy) has a function reached only through a table or an interface that is reported `unknown` (first phase) or `reached` with an `inferred` edge (second phase), a function nothing could call that is `unreachable`, and a known distance from an entry to a named function" is promised, and no story claims it for Go, Rust or Java/Groovy. The only reader-level end-to-end fixture is C: the table fixture in `story:reader-indirect-call-sites` and the same fixture in `story:reach-registered-candidates`. The other tests use synthetic documents. The requirements ask for the same thing: R7 "Golden tests per language with a known entry→sink distance" and the soundness test "a sink reachable ONLY through a dispatch table must come back `unknown-indirect` (phase 1) or `reached` with an `inferred` edge (phase 2)". The natural homes are `story:reader-indirect-call-sites` for phase 1 and `story:reach-registered-candidates` for phase 2. Its outcome narrows only `table`, `fn-pointer`, `callback` and `trait-object`, so the `closure` and `interface` kinds that Go and Java need for the "interface" half of the promise are not claimed in phase 2 either — .engineering/planning/epic/sound-reach.md:70
- story:reach-framework-entries — "A call to a GORM dynamic finder is an `indirect` site of kind `dynamic-name`" claims an R3 reader outcome (phase 1 in the requirements). `story:reader-indirect-call-sites` already claims the Groovy reader's `dynamic-name` sites (the "call by a computed name" sentence), and both stories own `adapters/groovy`, so two items claim one outcome. The sentence should be deleted from this story or the indirect-sites story should name the GORM finder explicitly — .engineering/planning/story/reach-framework-entries.md:20
- story:reader-indirect-call-sites — R3 lists for C "callback registration" as an indirect kind, but the C reader sentence names only a function pointer, a struct member and an array of structs. `callback` is in the kind list and `story:reach-registered-candidates` consumes it, yet no reader is stated to emit it and no per-kind fixture is promised for it. The body should say which reader gives `callback` — .engineering/planning/story/reader-indirect-call-sites.md:19
- story:reach-registered-candidates — R6 asks each result record to carry `via_indirect` and `inferred_edges:[...]`. This story says each inferred edge "carries the position that stores the function" but not where it is reported. `story:reach-three-valued` has no `inferred` marker among its `via` kinds, and `story:reach-at-positions` returns only status, distance, nearest entry and `via_unknown`. The outcome should state that the `reach/2` report and the `--at` records list the inferred edges with their evidence positions — .engineering/planning/story/reach-registered-candidates.md:16

What I read: 7 artifacts (the epic and its 6 stories) in full, through `aep plan artifact show` for each and `aep plan artifact graph`. I also read `story:reachability`, `story:reader-function-spans` and `story:readers-bound-untrusted-input` for what already exists, and the requirements document.

The parent list: I extracted 24 promises from the requirements (R1 to R7, phasing and fit). 20 trace fully to a story or to the epic's "What exists", 2 trace partly (C callback registration; `inferred_edges` in the output) and 2 trace to nothing (R7 per-language golden tests, and the per-language dispatch-table soundness test).

Reach beyond the parent: I found none. The `--format 1` compatibility, the `not read` answer for a refused function, the `*` pattern and the refusal of an entry line that matches no function all follow from the epic's outcome or its no-silent-guessing rule.

What I could not establish:
- The rename of R4's `unknown-indirect` to `unknown` is a deviation from the requirements that the epic does not record. It is not a coverage defect, so I left it out of the findings.
- The R1 Rust "`main`/exported-handler detection": the Rust starter file in `story:reach-entry-file` has no stated contents, so I could not tell whether this is claimed. This is thin, not a finding.
- Whether the assertions in `story:reach-three-valued` can be checked, and the dependency ordering between stories, belong to the acceptance, design and parallel-safety critics.

```findings
- file: .engineering/planning/epic/sound-reach.md
  line: 70
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic's Done When promises a checked-in example per reader language (Go, Rust, C, Java/Groovy) with an unknown-then-inferred-reached table or interface function, an unreachable dead function and a known distance (R7 golden and soundness tests), and no story claims it beyond the C table fixture; reader-indirect-call-sites (phase 1) and reach-registered-candidates (phase 2, which also leaves closure and interface kinds unnarrowed) are the natural homes"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 20
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the sentence making a GORM dynamic finder an indirect site of kind dynamic-name claims the same Groovy-reader outcome as reader-indirect-call-sites' computed-name sites, and it is an R3 phase-1 outcome placed in a phase-3 story, so two items claim one outcome"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 19
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "R3's C callback registration is not claimed: the C reader sentence names function pointer, struct member and array of structs only, so no reader is stated to emit kind callback although reach-registered-candidates consumes it"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 16
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "R6's per-record inferred_edges is claimed by no story: the outcome says each inferred edge carries a position but not that the reach/2 report and the --at records print them, and reach-at-positions' record carries only status, distance, nearest entry and via_unknown"
```
