---
format: aep.planning-md/3
id: review-result:ecma-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, epic:ecmascript-to-native
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
story:native-run-harness — the test compares two whole records for byte equality, but the record is specified to hold the wall time in nanoseconds and the node CPU model, so the acceptance cannot be met as written; say which fields the test compares (record minus wall time) — .engineering/planning/story/native-run-harness.md:52 against :72-74
story:wasm-writer — `codespec harness` runs 200 seeds of 300 commands byte-equal "where a driver exists", but no wasm driver exists under examples/*/drivers (only go, rust, rust-original) and the story writes none, so the check passes vacuously; name the driver the story adds or drop the clause — .engineering/planning/story/wasm-writer.md:95
story:x86-64-writer — the same `codespec harness` 200 seeds clause "where a driver exists" has no x86-64 driver in examples/series/drivers or examples/inventory/drivers and the story adds none, so it holds on an empty set; name the driver the story adds or drop the clause — .engineering/planning/story/x86-64-writer.md:78
story:ecmascript-reader — the outcome is that `codespec read --lang ecmascript <file.js>` prints the module, but no Done When item runs that command (tests call the library and `--describe`), so the CLI wiring in crates/cli can be absent and every item still pass; add a check that the command prints fib.read.cir.json and exits 1 with the refusal for eval — .engineering/planning/story/ecmascript-reader.md:42 against :102-130
story:ecmascript-run-oracle — the outcome is that `codespec conform` can compare the runner with expected.txt, but no Done When item runs `codespec conform` for the ecmascript runner, so the crates/cli role registration in scope is never observed; add one with the command and the expected result — .engineering/planning/story/ecmascript-run-oracle.md:34 against :43-62
story:ecmascript-number-semantics — "a value outside the 64-bit range is defined by the case" states no result and none of the near-miss cases lists such a value, so a reader cannot tell what the case must print; state the result for an out-of-range float to int and add it to the near-miss list — .engineering/planning/story/ecmascript-number-semantics.md:63-65
story:ecmascript-swell-surface — the surface check reads swell surface.txt "fetched once per run and cached under target/" in one bullet and "the swell sources it reads come from the Cargo checkout, not a download" in another, so two incompatible data sources are acceptable; name the one the check reads — .engineering/planning/story/ecmascript-swell-surface.md:67 against :75-76
story:native-speed-comparison — the refusal to run on the workstation or on a node whose CPU model differs from the recorded one is stated in Method but no Done When item or near miss checks it, so the safeguard the Why depends on can be missing at close; add a test that the host name and CPU model checks refuse — .engineering/planning/story/native-speed-comparison.md:72-74 against :89-109
story:native-speed-comparison — "the report is committed with the run commit" requires no particular cell, so a report with one program and one engine closes the story; state which programs of The Set and which engines must have a measured or explicitly marked cell — .engineering/planning/story/native-speed-comparison.md:97
epic:ecmascript-to-native — "one command each for read, write, build and run" and "a result line" name no command for build or run and no story provides a build command (codespec-run-wasm and codespec-run-x86-64 assemble and run in one call), so the check cannot be repeated; name the four commands and the result line format — .engineering/planning/epic/ecmascript-to-native.md:75-77
Read: 9 artifacts of 9 given (epic:ecmascript-to-native and the 8 stories), full bodies with cat -n from the worktree files at cc42781d; ran aep plan artifact kinds and lifecycle story, grep and ls on examples/*/drivers and harness.toml, slot run --help, story:readers-bound-untrusted-input.
Could not establish: whether the sibling stories take a swell release tag that exists yet (out of my lane, scope/design); the test-binary naming of existing conformance commands such as codespec conform --role run for a new language, not read in crates/cli; Done When is the acceptance heading of 131 existing stories and is taken as the acceptance section, and its multi-item lists as the repository convention, not as a more-than-one-statement defect; the microvm refusal test names "the story that would add it" while no such story is drafted (design/scope lane).
```findings
- file: .engineering/planning/story/native-run-harness.md
  line: 52
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test compares two whole records for byte equality, but the record is specified to hold the wall time in nanoseconds and the node CPU model, so the acceptance cannot be met as written; say which fields the test compares"
- file: .engineering/planning/story/wasm-writer.md
  line: 95
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "`codespec harness` runs 200 seeds of 300 commands byte-equal where a driver exists, but no wasm driver exists and the story writes none, so the check passes vacuously"
- file: .engineering/planning/story/x86-64-writer.md
  line: 78
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the `codespec harness` 200 seeds clause where a driver exists has no x86-64 driver and the story adds none, so it holds on an empty set"
- file: .engineering/planning/story/ecmascript-reader.md
  line: 42
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the outcome is that `codespec read --lang ecmascript <file.js>` prints the module, but no Done When item runs that command, so the CLI wiring can be absent and every item still pass"
- file: .engineering/planning/story/ecmascript-run-oracle.md
  line: 34
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome is that `codespec conform` can compare the runner with expected.txt, but no Done When item runs `codespec conform` for the ecmascript runner, so the crates/cli role registration is never observed"
- file: .engineering/planning/story/ecmascript-number-semantics.md
  line: 63
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a value outside the 64-bit range is defined by the case states no result and no near-miss case lists such a value, so a reader cannot tell what the case must print"
- file: .engineering/planning/story/ecmascript-swell-surface.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the surface check reads swell surface.txt fetched once per run and cached under target/ in one bullet and from the Cargo checkout, not a download, in another, so two incompatible data sources are acceptable"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the refusal to run on the workstation or on a node whose CPU model differs from the recorded one is stated in Method but no Done When item or near miss checks it"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 97
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the report is committed with the run commit requires no particular cell, so a report with one program and one engine closes the story; state which programs and engines must have a measured or marked cell"
- file: .engineering/planning/epic/ecmascript-to-native.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "one command each for read, write, build and run and a result line name no command for build or run and no story provides a build command, so the check cannot be repeated; name the four commands and the result line format"
```
