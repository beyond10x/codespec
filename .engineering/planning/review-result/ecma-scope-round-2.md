---
format: aep.planning-md/3
id: review-result:ecma-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, epic:ecmascript-to-native
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

story:x86-64-writer — the x86-64 target of codespec-bench (artifact, ELF runner command, AOT row) is claimed by no story: native-speed-comparison says "story:x86-64-writer adds its target afterwards" and excludes it from its own outcome, while x86-64-writer owns no crates/bench and its Done When has no bench target, so the epic summary promise "compared in speed" for x86-64 has no owner — .engineering/planning/story/x86-64-writer.md:111 (cites .engineering/planning/story/native-speed-comparison.md:50 and :121; epic .engineering/planning/epic/ecmascript-to-native.md:7)

What I read: 9 artifacts whole (epic and 8 stories) plus review-result:ecma-scope-round-1, from the worktree files with cat/sed/grep; epic read first and 11 promises extracted (read command on pinned swell; refusal with reason and file:line; write wasm; write x86-64; run in container through slot with byte compare against the evaluator; codespec-bench AOT rows in swell schema with no measuring by codespec; the eight stories implemented; four-command one-file path with a refused-script row; gate budget and no benchmarks in the gate; microVM named not built; decisions D1 to D4); 10 traced to an item, the x86-64 side of the speed rows is the untraced one. Round 1: the build/run gap is closed (native-speed-comparison build and run, epic Done When), the Wasm and x86-64 math and conversion mappings are claimed (wasm-writer.md and x86-64-writer.md Done When), the QuickJS and swell engine list is gone from the codespec story; those three findings are fixed. No reach beyond the parent found.

What I could not establish: none. Not findings: the x86-64 writer refuses lambda, call_value and run-time strings while the reader produces them (x86-64-writer.md Refused list), drafter-named, so a closure script has no x86-64 artifact; shipping codespec-bench in the release archive (native-speed-comparison Done When) is not stated in the epic but serves "codespec supplies its AOT rows", so I count it as in scope. Out of my lane: the `codespec write --lang wasm` wiring has no test of its own in wasm-writer Done When (acceptance); shared Cargo.toml, Taskfile.yml and check-parts.txt across all stories (parallel-safety).

```findings
- file: .engineering/planning/story/x86-64-writer.md
  line: 111
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the x86-64 target of codespec-bench (artifact, ELF runner command, AOT row) is claimed by no story: native-speed-comparison says story:x86-64-writer adds its target afterwards and excludes it from its own outcome, while x86-64-writer owns no crates/bench and its Done When has no bench target, so the epic summary promise compared in speed for x86-64 has no owner"
```
