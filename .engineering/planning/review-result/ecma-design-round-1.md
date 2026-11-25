---
format: aep.planning-md/3
id: review-result:ecma-design-round-1
kind: review-result
status: active
title: Design critic, round 1, epic:ecmascript-to-native
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
story:ecmascript-reader — its acceptance requires a conforms: true report with summary.pass of at least 25, but codespec conform runs step 1 of every reader case through codespec-run-ecmascript, which story:ecmascript-run-oracle creates and which depends_on this story, so the acceptance cannot be checked until a story that waits for it lands; the body must move the conforms: true check to story:ecmascript-run-oracle (or the two stories merge, and an edge from the reader to the runner is the alternative that makes a cycle) — .engineering/planning/story/ecmascript-reader.md:115-120, crates/conformance/src/conform.rs:66-67, .engineering/planning/story/ecmascript-run-oracle.md:10-11
story:ecmascript-swell-surface — its acceptance reads the reader's use of swell_syntax paths (swell-surface.txt is generated from them) and puts position() and the refusal-with-slice path in the reader crate, yet its scope owns only Cargo.toml and swell-surface.txt and says the crate's source is story:ecmascript-reader's, which depends_on it, so half of one abstraction sits in each and neither can be shown alone; the body must either own a source file for position() and the refusal mapping and hand-maintain the list, or fold into the reader — .engineering/planning/story/ecmascript-swell-surface.md:65-74, .engineering/planning/story/ecmascript-swell-surface.md:80-81
story:wasm-writer — the externs math.Sqrt, math.Abs, math.Floor, math.Mod that story:ecmascript-number-semantics defines as a mapping in each writer that claims floats belong to no Done When here (the manifest claim lists float but not intrinsic.math) while story:native-speed-comparison needs sqrt for nbody and spectral-norm; the body must claim intrinsic.math and map the four externs — .engineering/planning/story/wasm-writer.md:87-90, .engineering/planning/story/ecmascript-number-semantics.md:66-68
story:x86-64-writer — the same four math externs (sqrtsd, a floor, and an fmod) are in no Done When although its outcome takes floats because every benchmark needs them, and story:ecmascript-number-semantics cannot deliver them for a writer that does not exist yet; the body must claim intrinsic.math and map them — .engineering/planning/story/x86-64-writer.md:47-50, .engineering/planning/story/ecmascript-number-semantics.md:66-68
story:native-speed-comparison — its outcome and Method put an x86-64 column in the report, which reads the output of codespec-run-x86-64 that story:x86-64-writer produces, but the only edge is informed_by while the story calls itself last, so a wave may run it beside the writer and the epic's committed report then lacks the target; the relation to story:x86-64-writer must be depends_on, or the body must state that the x86-64 column is added by a later run and drop it from the outcome — .engineering/planning/story/native-speed-comparison.md:66-67, .engineering/planning/story/native-speed-comparison.md:12

What I read: 9 artifacts (the epic and 8 stories, whole bodies), aep plan artifact relations, graph, validate (valid, only stale review-result notices), conform.rs doc of the reader steps; walked all 27 declared edges including edges to artifacts outside the set (cir1-arrays-floats, cir1-type-test, readers-bound-untrusted-input, reader-asm, low-level-readers) and found no cycle among declared edges; traced 12 acceptance reads to a producer in the set, 10 recorded by an edge (directly or transitively), 2 not (the reader to the oracle runner, the speed comparison to the x86-64 runner); the reader-to-swell-surface edge is recorded but the reverse read in swell-surface is not.
What I could not establish: none that sets the verdict. Out of my lane: the oracle story adds -0, NaN and int 32 cases to a corpus the reader also owns (ownership of those case files, parallel safety); the x86-64 depends_on wasm-writer is a recorded shared-file order (crates/cli, crates/conformance, Taskfile.yml) and is not a finding.

```findings
- file: .engineering/planning/story/ecmascript-reader.md
  line: 115
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance requires a conforms: true report with summary.pass of at least 25, but codespec conform runs step 1 of every reader case through codespec-run-ecmascript, which story:ecmascript-run-oracle creates and which depends_on this story, so the acceptance cannot be checked until a story that waits for it lands; the body must move the conforms: true check to story:ecmascript-run-oracle or the two stories merge"
- file: .engineering/planning/story/ecmascript-swell-surface.md
  line: 65
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the reader's use of swell_syntax paths and puts position() and the refusal-with-slice path in the reader crate, yet its scope owns only Cargo.toml and swell-surface.txt and the crate source is story:ecmascript-reader's, which depends_on it, so half of one abstraction sits in each; the body must own a source file for position() and the refusal mapping and hand-maintain the list, or fold into the reader"
- file: .engineering/planning/story/wasm-writer.md
  line: 87
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the externs math.Sqrt, math.Abs, math.Floor, math.Mod that story:ecmascript-number-semantics defines as a mapping in each writer that claims floats belong to no Done When here (the manifest claim lists float but not intrinsic.math) while story:native-speed-comparison needs sqrt; the body must claim intrinsic.math and map the four externs"
- file: .engineering/planning/story/x86-64-writer.md
  line: 47
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the same four math externs are in no Done When although its outcome takes floats because every benchmark needs them, and story:ecmascript-number-semantics cannot deliver them for a writer that does not exist yet; the body must claim intrinsic.math and map them"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 66
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome and Method put an x86-64 column in the report, which reads the output of codespec-run-x86-64 that story:x86-64-writer produces, but the only edge is informed_by while the story calls itself last; the relation must be depends_on, or the body must state the x86-64 column is added by a later run and drop it from the outcome"
```
