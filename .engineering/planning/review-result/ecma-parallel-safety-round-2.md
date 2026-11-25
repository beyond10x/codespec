---
format: aep.planning-md/3
id: review-result:ecma-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic:ecmascript-to-native
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

story:native-speed-comparison — it shares root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md` and `CHANGELOG.md` (cited, both Scope sections) with story:x86-64-writer, only an `informed_by` edge links them (no `depends_on` either way), its Scope says "this story last" and in the same paragraph that `story:x86-64-writer` "adds its target afterwards", `aep plan artifact waves` puts the benchmark in wave 39 and the writer in wave 40, and the writer's Shares sentence does not name the benchmark; remedies: a `depends_on` edge recording the shared files as its reason (either direction, stated in both bodies), or splitting the surface so they no longer share it — .engineering/planning/story/native-speed-comparison.md:119-122, .engineering/planning/story/x86-64-writer.md:115-116

Round 1 findings: all four are resolved. The oracle against both writers is now ordered by `depends_on` (wasm-writer depends on the oracle, x86-64-writer on wasm-writer, wasm-writer.md:13) and named in each Shares sentence. The harness shares AGENTS.md and CHANGELOG.md with number-semantics through a `depends_on` edge (native-run-harness.md:10, 96-97). swell-surface now depends on the harness (ecmascript-swell-surface.md:10, 97-99). The `containers/` directory is split per story (base, node, wasm, x86-64; native-run-harness.md:57-63).

What I read: 9 artifacts (the epic and 8 stories), each body read whole, with `aep plan artifact waves` (the 8 stories sit in waves 2, 21, 22, 31, 33, 36, 39, 40; every pair inside the set is store-marked inferred, though each body lists the same paths in its Scope, so I count them cited), `aep plan artifact relations --help`, and `cat` of the round 1 result. Surfaces: 8 cited (body Scope sections), 0 inferred, 0 unplaceable (the epic itself lands nowhere). The `depends_on` edges form one chain, number-semantics, harness, swell-surface, reader, oracle, wasm-writer, then x86-64-writer; native-speed-comparison hangs off wasm-writer and the oracle. The only pair of the 28 without a `depends_on` path between its members is x86-64-writer with native-speed-comparison. Acceptance reads traced to a producer in the set: 13; 11 recorded by a direct `depends_on` edge, 2 only transitively (the oracle reading number-semantics float encoding, x86-64-writer reading number-semantics kit cases), and the benchmark reading x86-64-writer output is `informed_by` only, where its body says that column is absent until the writer exists, so it is not a finding.

What I could not establish: no path is unplaced. Out of my lane, not counted in the verdict: story:ecmascript-swell-surface cannot be implemented until a swell release that does not exist yet (ecmascript-swell-surface.md:65-67), a blocker on the reader, oracle, both writers and the benchmark that no edge in this store can record; the story:ecmascript-swell-surface Scope names the whole `scripts` directory while its Own list names only `scripts/check-parts.txt` (a scope narrowing for the design or scope critic, harmless here because every story that shares it is ordered before or after it by an edge). The benchmark also says the x86-64 target is "not part of this story's outcome" while x86-64-writer's Scope names no `crates/bench` change, so no story owns adding that target (scope lane). Collisions with stories outside the set (for example story:builtin-defect-checks on `crates/cli/src/lib.rs`) were not assessed.

```findings
- file: .engineering/planning/story/native-speed-comparison.md
  line: 119
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares root Cargo.toml, Cargo.lock, Taskfile.yml, scripts/check-parts.txt, AGENTS.md and CHANGELOG.md with story:x86-64-writer, only an informed_by edge links them, its Scope says this story last while also saying x86-64-writer adds its target afterwards, aep plan artifact waves puts it in wave 39 before the writer in wave 40, and the writer Shares sentence at x86-64-writer.md:115 does not name it; add a depends_on edge recording the shared files in both bodies, or split the surface"
```
