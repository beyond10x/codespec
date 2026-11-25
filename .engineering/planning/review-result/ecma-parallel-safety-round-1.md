---
format: aep.planning-md/3
id: review-result:ecma-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic:ecmascript-to-native
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

story:ecmascript-run-oracle — it lands on root `Cargo.toml`, `Cargo.lock`, `crates/cli` and `scripts/check-parts.txt` (cited, its Scope) with both story:wasm-writer and story:x86-64-writer, no depends_on edge runs between the oracle and either writer (each depends on the reader and the harness, not on each other), and neither body names the oracle; remedies: an ordering edge recording the shared files as its reason, or splitting the surface so they no longer share it — .engineering/planning/story/ecmascript-run-oracle.md:69, .engineering/planning/story/wasm-writer.md:122
story:native-run-harness — it shares `AGENTS.md` and `CHANGELOG.md` (cited, its Scope line 84) with story:ecmascript-number-semantics and story:ecmascript-reader, no depends_on edge orders them, and its sharing sentence names only `Taskfile.yml` and `scripts/check-parts.txt`, so those two pairs read as independent; remedies: an ordering edge recording the files as its reason, or splitting the surface — .engineering/planning/story/native-run-harness.md:84-86, .engineering/planning/story/ecmascript-number-semantics.md:84
story:ecmascript-swell-surface — it shares root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml` and `scripts/check-parts.txt` (cited, its Scope) with story:native-run-harness, no depends_on edge runs either way, its sharing sentence names only story:ecmascript-reader, and both it ("this story first") and the harness ("this story first") claim first place on those files; remedies: an ordering edge recording the shared files as its reason, or splitting the surface — .engineering/planning/story/ecmascript-swell-surface.md:83-84, .engineering/planning/story/native-run-harness.md:86-87
story:native-run-harness — its Done When has this story build the images that pin `wasm-tools`, `wasmtime`, `node`, `binutils` and `cc` from `containers/<name>/Containerfile`, while story:wasm-writer, story:x86-64-writer and story:native-speed-comparison each list creating the Wasm, `as`/`ld`/`cc` and engine image files in the same `containers/` directory, so the body does not say which of the two creates each Containerfile (inferred: no Containerfile path is named in any body, and `containers/` does not exist in the tree yet); remedies: an ordering edge recording `containers/` as the reason with the harness limited to a stub image, or splitting the directory per story — .engineering/planning/story/native-run-harness.md:55-57, .engineering/planning/story/wasm-writer.md:117-118

What I read: 9 artifacts (the epic and 8 stories) with `cat` on each body, `aep plan artifact waves` (collision lines filtered to the set; every pair is inferred-marked because the store records all scope as confidence inferred, though each body lists the same paths in its own Scope section, so I count them cited), and `ls` of the tree for `containers/`, `Taskfile.yml`, `scripts/check-parts.txt`. Surfaces: 8 cited (body Scope sections), 0 inferred beyond that, 0 unplaceable. Acceptance reads traced to a producer in the set: 12; 10 recorded by a direct depends_on edge, 1 by a transitive edge only (the oracle reading the number-semantics float encoding, via the reader), 1 by informed_by only (native-speed-comparison reading story:x86-64-writer output; its body states the column is absent until the writer exists, so I did not report it).
What I could not establish: no Containerfile path is named anywhere, so the containers/ finding rests on an inferred surface. Out of my lane, not counted in the verdict: story:ecmascript-swell-surface cannot start until a swell release that does not exist yet (an outside dependency no edge can record), which blocks the reader, oracle, both writers and the benchmark; scope critic or operator may want it as a stated blocker. The collisions with stories outside the set (for example story:builtin-defect-checks on `crates/cli/src/lib.rs`) were not assessed.

```findings
- file: .engineering/planning/story/ecmascript-run-oracle.md
  line: 69
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it lands on root Cargo.toml, Cargo.lock, crates/cli and scripts/check-parts.txt with story:wasm-writer and story:x86-64-writer, no depends_on edge orders it against either writer, and no body names the oracle in that sharing; add an ordering edge recording the shared files, or split the surface (the surface comes from each body Scope section, the store records it as inferred)"
- file: .engineering/planning/story/native-run-harness.md
  line: 86
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares AGENTS.md and CHANGELOG.md with story:ecmascript-number-semantics and story:ecmascript-reader, no depends_on edge orders those pairs, and its sharing sentence names only Taskfile.yml and scripts/check-parts.txt; add an ordering edge recording the files, or split the surface"
- file: .engineering/planning/story/ecmascript-swell-surface.md
  line: 83
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares Cargo.toml, Cargo.lock, Taskfile.yml and scripts/check-parts.txt with story:native-run-harness, no depends_on edge runs either way, its sharing sentence names only the reader, and both bodies say this story first; add an ordering edge recording the shared files, or split the surface"
- file: .engineering/planning/story/native-run-harness.md
  line: 55
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Done When builds images pinning wasm-tools, wasmtime, node, binutils and cc under containers/, while story:wasm-writer, story:x86-64-writer and story:native-speed-comparison each list creating image files in the same directory, and the body does not say which creates each Containerfile; this rests on an inferred surface (no Containerfile path is named and containers/ does not exist yet)"
```
