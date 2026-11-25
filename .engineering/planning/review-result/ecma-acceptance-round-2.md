---
format: aep.planning-md/3
id: review-result:ecma-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, epic:ecmascript-to-native
relations:
- reviews: epic:ecmascript-to-native
- reviews: story:ecmascript-number-semantics
- reviews: story:ecmascript-swell-surface
- reviews: story:ecmascript-reader
- reviews: story:native-run-harness
- reviews: story:ecmascript-run-oracle
- reviews: story:wasm-writer
- reviews: story:x86-64-writer
- reviews: story:native-speed-comparison
revision: 1
---
needs-revision
story:native-run-harness — the Outcome promises a returned "digest of everything that decided the result" but the record field list names no such digest (only image digest and stdout digest), and "not in the digest of the output" does not say which digest, so the promised value cannot be checked; name the field and its inputs in the record — .engineering/planning/story/native-run-harness.md:35 against :81-84
story:native-speed-comparison — the near miss "a script with two top-level functions of the same name" states no result (refused? which reason? which row?), so a test for it can pass with any behaviour; state the row or refusal it gives, as the sibling near misses do — .engineering/planning/story/native-speed-comparison.md:109
Read: 9 of 9 ids (epic:ecmascript-to-native and the 8 stories), full bodies via `aep plan artifact show <id>`, plus review-result:ecma-acceptance-round-1; checked examples/*/drivers, crates/cli for --calls and the reader/writer roles, and the ToInt32 value 1e20 -> 1661992960 by computation. Round 1: all 10 findings are fixed (harness compares record minus wall_ns; wasm and x86-64 state no harness driver exists and none is claimed; reader and oracle run the `codespec read` and `codespec conform` commands in crates/cli tests; out-of-range float to int is a named stop in the kit; swell surface check reads one source, the Cargo checkout; speed-comparison was redesigned so the refusal-on-workstation clause and one-cell report are gone, replaced by a `read N of M` count and a five-kernel floor; the epic names its four commands and the result line).
Could not establish: whether a kit case can hold a named evaluator stop in expected.txt (number-semantics, line 50-54; design/scope lane); the x86-64 story says it "names the baseline" for roundsd (SSE4.1) but names none, a loose end I judged not to change the check; whether the swell release tags the swell-surface and speed-comparison stories pin exist (out of my lane, scope/design); Done When is taken as the acceptance heading and its multi-item lists as the repository convention.
```findings
- file: .engineering/planning/story/native-run-harness.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises a returned digest of everything that decided the result but the record field list names no such digest (only image digest and stdout digest), and \"not in the digest of the output\" does not say which digest, so the promised value cannot be checked; name the field and its inputs in the record"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 109
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the near miss a script with two top-level functions of the same name states no result (refused? which reason? which row?), so a test for it can pass with any behaviour; state the row or refusal it gives"
```
