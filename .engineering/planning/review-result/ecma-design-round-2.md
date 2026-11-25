---
format: aep.planning-md/3
id: review-result:ecma-design-round-2
kind: review-result
status: active
title: Design critic, round 2, epic:ecmascript-to-native
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
story:ecmascript-swell-surface — it owns src/swell.rs, tests/swell.rs and the crate Cargo.toml (and the root members entry) but not src/lib.rs, main.rs or tests/main.rs, which story:ecmascript-reader owns and which waits for it, so the crate has no target to build and its position() and surface tests cannot run alone, and tests/swell.rs beside tests/main.rs makes a second integration binary; the body must own a minimal src/lib.rs (mod swell) and have tests/swell.rs declared as a module of tests/main.rs, with the reader extending both — .engineering/planning/story/ecmascript-swell-surface.md:93-95, .engineering/planning/story/ecmascript-reader.md:145-147
story:native-speed-comparison — it drops the x86-64 target from its outcome but its Scope says "story:x86-64-writer adds its target afterwards" while story:x86-64-writer owns no crates/bench path and has no Done When for a bench target, so nothing in the set adds the target; the body must say a later story (not in this epic) adds it, or story:x86-64-writer must take the target into its scope and Done When — .engineering/planning/story/native-speed-comparison.md:121, .engineering/planning/story/x86-64-writer.md:109-118

What I read: 9 artifacts (the epic and 8 stories, whole bodies) and review-result:ecma-design-round-1, with aep plan artifact relations, graph and validate (valid; only stale review-result notices). I walked 28 declared edges, including those to artifacts outside the set (story:cir1-arrays-floats, story:cir1-type-test, story:readers-bound-untrusted-input, story:reader-asm, epic:low-level-readers), found no cycle, and no edge from those back into the set. I traced 14 acceptance reads to a producer in the set; 13 are recorded by an edge (directly or transitively), and 1 is not (the swell-surface reading of lib.rs, the finding above). Round 1: the reader/oracle conform check moved to the oracle, intrinsic.math is claimed in both writers, and the swell-surface source file src/swell.rs is now owned, so those three are fixed; the speed-comparison edge was resolved by dropping x86-64 from the outcome, which leaves the second finding.
What I could not establish: none that sets the verdict. The set is a near-linear chain (number-semantics, harness, swell-surface, reader, oracle, wasm, x86-64), but each edge has its shared-file reason written beside it (AGENTS.md, CHANGELOG.md, Cargo.toml, Taskfile.yml, crates/cli, crates/conformance), so it is the file-sharing trade-off and not a finding. Out of my lane: whether the corpus ecmascript reader cases need a registered report before task check passes (acceptance), and whether the oracle and reader both adding cases in conformance/reader/ecmascript collide (parallel safety).

```findings
- file: .engineering/planning/story/ecmascript-swell-surface.md
  line: 93
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it owns src/swell.rs, tests/swell.rs and the crate Cargo.toml (and the root members entry) but not src/lib.rs, main.rs or tests/main.rs, which story:ecmascript-reader owns and which waits for it, so the crate has no target to build and its position() and surface tests cannot run alone, and tests/swell.rs beside tests/main.rs makes a second integration binary; the body must own a minimal src/lib.rs (mod swell) and have tests/swell.rs declared as a module of tests/main.rs, with the reader extending both"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 121
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it drops the x86-64 target from its outcome but its Scope says story:x86-64-writer adds its target afterwards while story:x86-64-writer owns no crates/bench path and has no Done When for a bench target, so nothing in the set adds the target; the body must say a later story outside this epic adds it, or story:x86-64-writer must take the target into its scope and Done When"
```
