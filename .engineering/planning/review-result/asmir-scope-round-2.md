---
format: aep.planning-md/3
id: review-result:asmir-scope-round-2
kind: review-result
status: active
title: 'asm_ir plan: scope critic, round 2'
relations:
- reviews: story:asm-ir-model
- reviews: story:asm-ir-lower-core
- reviews: story:asm-x86-64-backend
- reviews: story:asm-x86-64-writer
- reviews: story:asm-aarch64-backend
- reviews: story:asm-riscv64-backend
- reviews: story:asm-kit-conformance
- reviews: story:asm-ir-lower-slices-strings-errors
- reviews: story:asm-ir-lower-floats
- reviews: story:asm-ir-lower-maps
- reviews: story:asm-ir-interpreter
- reviews: story:x86-64-writer
- reviews: epic:asm-ir-native-backends
revision: 1
---
needs-revision

story:asm-kit-conformance — "one site page links `docs/asm-ir.md`" (Outcome and Scope "one site page") is work no sentence of the epic asks for: the epic's Outcome and Done When name no site page, and the support matrix and `status.json` entries are generated from the writers' manifests and conform reports, so only the hand-written page is extra; drop the page or put it in the epic — .engineering/planning/story/asm-kit-conformance.md:40 (epic: .engineering/planning/epic/asm-ir-native-backends.md:10-33, 47-54)

Round 1 (review-result:asmir-scope-round-1), finding by finding:
- CI qemu install claimed three times: fixed. `story:asm-kit-conformance` now says "the emulators are installed by the backend stories" and no longer lists `scripts/ci-tools.sh` in Scope. Only the two backend stories claim it.
- `story:x86-64-writer` Done When and Scope duplicating `story:asm-kit-conformance`: fixed. The body is rewritten and says the writer is "that epic's (`story:asm-x86-64-writer`, `story:asm-ir-lower-floats`)". Scope is now `crates/bench`, `containers/x86-64`, `crates/asm-ir`, `crates/lower`, `adapters/asm/src/x86_64.rs`, `adapters/asm-run`, `Taskfile.yml`, `CHANGELOG.md`. `adapters/x86-64`, `adapters/x86-64-run`, `crates/conformance` and `crates/docs` are gone, and so is any `conforms: true` bullet.
- `story:x86-64-writer` keeping System V, a C ABI and an assembly runtime: fixed. It now cites A4/A5 (no libc, no C driver), a private calling convention and an arena panic with exit 3. The "C-ABI entry point" is called "a later, additive `abi: "c"`", which matches the epic's out-of-scope line.
- `intrinsic.math` claimed by no story: fixed. `story:x86-64-writer` now owns `crates/asm-ir`, `crates/lower` and `adapters/asm/src/x86_64.rs` for it. The epic's Out of Scope also names it: "`intrinsic.math` of `epic:ecmascript-to-native`: `story:x86-64-writer` builds them on `codespec-write-x86-64` from this epic." So it is sanctioned, not reach.
- Nothing is left from round 1. The finding above is new this round.

What I read: 13 artifacts, with `aep plan artifact show` on `epic:asm-ir-native-backends`, its 11 stories, `story:x86-64-writer`, `epic:ecmascript-to-native` and `review-result:asmir-scope-round-1`. I also ran `aep plan artifact graph` and read `docs/asm-ir.md` §§ 4 and 10 and the header of `crates/docs/src/support.rs`. I listed the epic's promises before reading any story, and the list has 12:
1. Any language runs as a static binary on three architectures through one IR.
2. The `asm_ir/1` crate.
3. `codespec lower`.
4. The three writers.
5. `codespec-run-asm` running a kit case on each.
6. Each writer held to the kit by `codespec conform`.
7. The three held against each other.
8. All 11 stories implemented.
9. 122 of 145 cases on x86-64, plus the byte-equal cross-check.
10. The interpreter prints `expected.txt` for the 78 group-1 cases.
11. Gate steps within 2 minutes and tests within 60 s.
12. The out-of-scope list (23 cases, register allocation, WebAssembly, bench).
I traced all 12 to an item, and the group arithmetic holds (78+7+11+2+13+4+7 = 122). The finding is a reach, not a gap. Nothing duplicates another item's outcome: `story:asm-x86-64-backend` owns the backend, the `--module` runner and the corpus, `story:asm-x86-64-writer` owns the cir/1 path and the `<dir> <calls.json>` runner mode, and `story:asm-kit-conformance` owns the other two writers and the cross-check.

What I could not establish (all outside my lane, none set the verdict):
- `story:x86-64-writer` adds four ops (`fsqrt`, `fabs`, `ffloor`, `ftrunc`) to `asm_ir/1`. `story:asm-x86-64-backend` promises "all 41 operations" and a test that fails when an `asm_ir::Op` has no corpus module. `story:x86-64-writer` Scope omits `crates/asm-ir/tests/ops/`, so no item claims corpus modules for the new ops (design or parallel-safety lane).
- The AArch64 and RISC-V backend Done When says "every module of `index.txt` prints its expected output". That clashes with their refusal of `float.math` (design lane).
- The groups 2 to 7 stories say "the writers' manifests claim the features" (one manifest shared by three writers). `story:asm-kit-conformance` states only group 1 for AArch64 and RISC-V 64, and no story states that those two conform parts pass once later groups are claimed (acceptance or design lane).
- `epic:ecmascript-to-native` still says "x86-64 GNU assembly (System V ABI, Linux)" at its line 21. That is stale against the design and outside this set.
- Two round-1 notes still stand: the epic's "each moves the count" wording for the interpreter has gone from the epic, and the 23 later cases have no story, which the epic excludes.

```findings
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 40
  category: scope
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "'one site page links docs/asm-ir.md' (Outcome and Scope 'one site page') is not traceable to any sentence of epic:asm-ir-native-backends, which names no site page; the support matrix and status.json are generated from the writers' manifests and reports, so only the hand-written page is extra; drop it or add it to the epic"
```
