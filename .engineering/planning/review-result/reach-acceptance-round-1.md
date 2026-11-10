---
format: aep.planning-md/3
id: review-result:reach-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic on the reach epic, round 1
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

story:reach-entry-file — the Outcome says the report lists the entries-file line each entry came from, but the acceptance requires the file's report to be the same as the `--entry` flags' report, which carry no file line, so the two cannot both hold; the acceptance names no byte-level shape for the origin line — "gives the same report as the equivalent `--entry` flags" — .engineering/planning/story/reach-entry-file.md:26
story:reach-entry-file — `reach --entry` today takes only `<path>.<name>` or `<path>.<type>.<method>`, so "equivalent `--entry` flags" for a pattern and an annotation entry do not exist and the acceptance has no baseline to compare against — crates/cli/src/lib.rs:197
story:reach-three-valued — the acceptance relies on `--format 1`, a flag `reach` does not have today (its flags are `--entry`, `--exported`, `--dispatch`, `--report`), and the Outcome never says the flag is added; the "line saying how many of them are `unknown`" has no stated text, so a test cannot assert it — .engineering/planning/story/reach-three-valued.md:39
story:reach-three-valued — the Outcome's `unknown`-spreads rule, the extern-taking-a-function-value site, the function-without-a-body site, and `via` with `value` have no acceptance bullet, so each can be unbuilt and the story still closes — .engineering/planning/story/reach-three-valued.md:14
story:reader-indirect-call-sites — "one checked-in source file per kind that reader gives" is circular, because the reader decides which kinds it gives; the kinds are not mapped to readers (`callback` goes to no reader, and the Go "function-value" kind is not named), so "no reader gives fewer" cannot be checked — .engineering/planning/story/reader-indirect-call-sites.md:35
story:reader-indirect-call-sites — the `cir/1` field versus `cir/2` decision is left to a first step, yet no acceptance requires `docs/design.md` to record it or says how "accepts a document without `indirect`" reads if the version changes — .engineering/planning/story/reader-indirect-call-sites.md:33
story:reader-indirect-call-sites — "The conformance kit's reader cases are unchanged; no reader claims fewer features" names no command or file to run and compare, so it is a negation nobody can run the same way twice — .engineering/planning/story/reader-indirect-call-sites.md:42
story:reach-at-positions — the acceptance introduces the answer `not read` with the refusal's reason, which the Outcome does not list (it lists only status, `no function`, `via_unknown`), and neither body says where `reach` gets the refusal from — .engineering/planning/story/reach-at-positions.md:26
story:reach-at-positions — "answered for both" gives no output shape for a line in two functions (the Outcome has one `reach` field per record), and a line inside a body with no statement (blank, comment, closing brace) has no stated answer under "whose body holds that line" — .engineering/planning/story/reach-at-positions.md:28
story:reach-registered-candidates — the Outcome narrows `fn-pointer` and `callback` sites to a parameter and `trait-object` sites to the methods of types the reached code makes, but the acceptance tests only the C table fixture, so those two narrowings can be unbuilt; the "function of the same signature stored nowhere" is also not in the fixture it cites — .engineering/planning/story/reach-registered-candidates.md:25
story:reach-framework-entries — the Outcome's "a GORM dynamic finder is an `indirect` site of kind `dynamic-name`" and its "filters" entry have no acceptance bullet (the fixture holds two actions, one interceptor, one mapping target, no filter and no finder), and the acceptance does not say where the entries file is written — .engineering/planning/story/reach-framework-entries.md:24

What I read: 7 of 7 ids (epic:sound-reach and the six stories), with `aep plan artifact show` on each, plus `aep plan artifact kinds` and `lifecycle story`. I checked the paths named in the acceptances against the tree. `crates/cli/src/lib.rs` for the `reach` flags, and `crates/effects/tests/reach.rs` and `crates/cli/tests/reach.rs`, exist. `story:reachability` is implemented and `story:reader-function-spans` is draft.

What I could not establish:
- Whether `cir/1` records annotations of functions. I found only persistence and framework marks in `crates/model/src/lib.rs`, so "carrying a named annotation or attribute the reader recorded" may rest on a field that is not there. That is design and scope, outside my lane.
- Whether a refused function leaves any trace in a `cir/1` document. `crates/model/src/lib.rs:66` mentions a list of "could not read" constructs, so I did not call it missing.
- Every story's Acceptance lists three to seven independent outcomes as bullets, where the role's rule asks for one statement. I treated each bullet as checkable and did not raise this as a finding. Splitting them is the drafter's call.
- Out of my lane: `story:reader-indirect-call-sites` touches five adapters plus the model, which is a scope and size matter. The epic's Done When expects a Go, Rust, C and Java example with an `inferred` edge, but `story:reach-registered-candidates` has only the C fixture, which is a coverage matter.

```findings
- file: .engineering/planning/story/reach-entry-file.md
  line: 26
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the report lists the entries-file line each entry came from, but the acceptance requires the same report as the equivalent --entry flags, which carry no file line, so the two cannot both hold"
- file: crates/cli/src/lib.rs
  line: 197
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "reach --entry takes only <path>.<name> or <path>.<type>.<method>, so the equivalent --entry flags for a pattern and an annotation entry named in the acceptance do not exist and the comparison has no baseline"
- file: .engineering/planning/story/reach-three-valued.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance relies on a --format 1 flag that reach does not have today and the Outcome never introduces it, and the line counting unknown functions has no stated text to assert"
- file: .engineering/planning/story/reach-three-valued.md
  line: 14
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome's unknown-spreading rule, the extern-taking-a-function-value site, the function-without-a-body site and the value edge kind in via have no acceptance bullet"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "one source file per kind that reader gives is circular because the reader decides which kinds it gives, and the kind-to-reader mapping leaves callback and the Go function-value kind unassigned"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the cir/1-field versus cir/2 decision has no acceptance requiring docs/design.md to record it or saying how reading a document without indirect works if the version changes"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the conformance-kit bullet names no command or file to run and compare, so 'unchanged' and 'no reader claims fewer features' cannot be checked the same way twice"
- file: .engineering/planning/story/reach-at-positions.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance introduces the answer not read with the refusal's reason, which the Outcome does not list, and neither body says where reach gets the refusal from"
- file: .engineering/planning/story/reach-at-positions.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "answered for both gives no output shape for a line in two functions, and a line inside a body with no statement has no stated answer"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the fn-pointer, callback and trait-object narrowings in the Outcome have no acceptance bullet, only the C table fixture is tested, and the stored-nowhere function is not in the fixture it cites"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 24
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the GORM dynamic-finder dynamic-name site and the filters entry have no acceptance bullet, and the acceptance does not say where the entries file is written"
```
