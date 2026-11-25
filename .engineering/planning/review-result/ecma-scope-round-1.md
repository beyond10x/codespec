---
format: aep.planning-md/3
id: review-result:ecma-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, epic:ecmascript-to-native
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

epic:ecmascript-to-native — "One JavaScript benchmark file goes from source to a result line on the build node with one command each for read, write, build and run" is promised and no drafted item claims the build command (WAT to .wasm, assembly to ELF) or the run command for a single file, since the runners take a case with calls.json and `task bench` runs the whole set; native-speed-comparison would most naturally take it — .engineering/planning/epic/ecmascript-to-native.md:75
story:wasm-writer — the manifest list names no `sem.float.ieee`, `sem.float.to_int`, `sem.int.to_float` or `intrinsic.math`, and `story:ecmascript-number-semantics` can only map them in writers that already claim floats, so nothing claims the Wasm mapping of `Math.sqrt`/`Math.abs`/`Math.floor`/`math.Mod` and the `|0` conversions the reader produces — .engineering/planning/story/wasm-writer.md:87
story:x86-64-writer — the first cut names floats in `xmm` but no mapping for the `math.*` externs or the float to int and int to float conversions, so nothing claims what `nbody` and every `|0` form of the reader need on this target — .engineering/planning/story/x86-64-writer.md:66
story:native-speed-comparison — the engine list adds QuickJS and swell, which the epic does not ask for ("the fastest ECMAScript engines") and swell work is out of scope, so the body must either say why each belongs or drop them — .engineering/planning/story/native-speed-comparison.md:62

What I read: 9 artifacts (epic and 8 stories) whole, in the worktree files; `aep plan artifact graph` filtered for ecmascript/wasm/x86/native (no other item claims any part); `grep -rli` over planning for wasm, ecmascript, x86-64 (only low-level-readers, reader-asm, tree-sitter-go-reader, which are informed_by targets). I extracted 12 promises from the epic (read command on pinned swell; refusal with reason and file:line; write wasm; write x86-64; assembled and run in a container through slot; byte compare with evaluator; committed speed report on build node only; the eight stories implemented; one-file read/write/build/run with a report naming versions; gate budget and benchmarks outside the gate; microVM named not built; decisions D1 to D4) and traced 11 to an item; the one-file build/run command is the untraced one.

What I could not establish: none. Not findings, drafter-named: the x86-64 story refuses `lambda`, `call_value` and run-time strings (x86-64-writer.md:72-74) while the reader produces them (ecmascript-reader.md:75-78), so a closure script cannot reach x86-64; the story says so itself and the epic says nothing, a drafter-named narrowing I did not count as a gap. Out of my lane: acceptance checkability of `at least 25` pass and the hex float encoding (acceptance), the x86-64 dependency on wasm-writer and shared Taskfile/check-parts.txt files (design, parallel-safety).

```findings
- file: .engineering/planning/epic/ecmascript-to-native.md
  line: 75
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the promise of one command each for read, write, build and run of one benchmark file is claimed by no item: the runners take a case with calls.json and task bench runs the whole set; native-speed-comparison would most naturally take it"
- file: .engineering/planning/story/wasm-writer.md
  line: 87
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the manifest list names none of sem.float.ieee, sem.float.to_int, sem.int.to_float, intrinsic.math, so the Wasm mapping of the math externs and the int/float conversions is claimed by no item"
- file: .engineering/planning/story/x86-64-writer.md
  line: 66
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first cut names no mapping for the math.* externs or the float/int conversions, which the reader and nbody need, so the x86-64 side of that promise is claimed by no item"
- file: .engineering/planning/story/native-speed-comparison.md
  line: 62
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the engine list adds QuickJS and swell, which the epic does not ask for (the fastest ECMAScript engines) and swell work is out of scope; the body must say why each belongs or drop them"
```
