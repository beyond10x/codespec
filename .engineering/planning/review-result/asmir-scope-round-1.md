---
format: aep.planning-md/3
id: review-result:asmir-scope-round-1
kind: review-result
status: active
title: 'asm_ir plan: scope critic, round 1'
relations:
- reviews: story:asm-ir-model
- reviews: story:asm-ir-lower-core
- reviews: story:asm-x86-64-backend
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

story:asm-kit-conformance — "CI installs `qemu-user` for AArch64 and RISC-V 64 in the `check / tools` job" is also claimed by story:asm-aarch64-backend ("and CI installs it", scope owns `scripts/ci-tools.sh`) and story:asm-riscv64-backend ("CI installs it"), so three items will each be marked done for one outcome; drop it here and keep it in the backends, or the reverse — .engineering/planning/story/asm-kit-conformance.md:43
story:x86-64-writer — Done When still promises "`codespec conform --lang x86-64 --role writer` has `conforms: true`", and Scope still owns `adapters/x86-64`, `adapters/x86-64-run`, `crates/cli` (the `--lang` value), `crates/conformance` and `crates/docs`, which is the same outcome and the same files as story:asm-kit-conformance (`codespec-write-x86-64`, the three `--lang` values, `conforms: true`); the "Through asm_ir/1" section does not retire the old bullets or rewrite Scope — .engineering/planning/story/x86-64-writer.md:83
story:x86-64-writer — Outcome and Done When keep the System V AMD64 ABI, aggregates "passed as the ABI says", callee-saved registers, a libc-checked stack alignment and an assembly runtime, while the new section names only "two differences"; the epic excludes "a C-ABI entry point" and puts calling convention, runtime and libc elsewhere (A3 to A6), so the story still claims work the epic refuses — .engineering/planning/epic/asm-ir-native-backends.md:59 (story lines 49, 53, 74-75, 95)
story:x86-64-writer — `intrinsic.math` (`math.Sqrt` as `sqrtsd`, `roundsd`, `fprem`) is kept "here", but `asm_ir/1` has no `fsqrt` or other math ("later, additive") and the story's Scope owns none of `crates/asm-ir`, `crates/lower` or `adapters/asm`, so no story in the set claims the IR addition or the lowering it needs — .engineering/planning/story/x86-64-writer.md:88 (docs/asm-ir.md:102)

What I read: the epic, its 10 stories, story:x86-64-writer and docs/asm-ir.md, using `aep plan artifact show` for each artifact and `aep plan artifact graph`. I wrote the list of 9 promises from the epic before reading any story. The 9: asm_ir crate; lowering; three writers and the runner; conform and cross-check; the 78 cases of group 1 on three targets; groups 2 to 7 and the interpreter drafted with counts; the gate time limits; the out-of-scope list; the design link. All 9 trace to an item. There is no gap. The group sums check out: 78+7+11+2+13+4+7 = 122. Nothing reaches into WebAssembly, the bench, register allocation or a fourth architecture. Two stories reach into the epic's exclusions or other items' outcomes: story:asm-kit-conformance and story:x86-64-writer, per the findings above.

What I could not establish, all outside my lane:
- story:asm-ir-lower-floats states "115 in total" but depends only on story:asm-kit-conformance. The total assumes groups 2 to 5 have landed (design lane).
- story:asm-ir-lower-floats does not own the writer manifests in `adapters/asm`, yet its cases cannot pass in `codespec conform` until the manifests claim `float` (parallel-safety or design lane).
- story:asm-ir-interpreter depends only on story:asm-ir-model, yet its last acceptance bullet needs the lowering and the driver (design lane).
- The epic says "each moves the count of passing cases" of groups 2 to 7 "and the interpreter". The interpreter cannot move that count, and its story does not claim to. I read this as loose wording in the epic and did not raise a finding.
- The "later" 23 kit cases have no story. The epic's Done When does not ask for them and docs/asm-ir.md § 5 names them, so I did not call it a gap.
- Two x86-64-writer bullets contradict the runtime of docs/asm-ir.md § 6: "exit 4" on arena exhaustion against `rt.alloc` panicking "out of memory" (exit 3), and "no step budget". This is the design critic's matter.

```findings
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 43
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome 'CI installs qemu-user for AArch64 and RISC-V 64 in the check / tools job' is also claimed by story:asm-aarch64-backend and story:asm-riscv64-backend ('CI installs it', scripts/ci-tools.sh in scope), so three items claim one outcome; drop it from one side"
- file: .engineering/planning/story/x86-64-writer.md
  line: 83
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Done When still claims 'codespec conform --lang x86-64 --role writer has conforms: true' and Scope still owns adapters/x86-64, adapters/x86-64-run, the crates/cli --lang value, crates/conformance and crates/docs, the same outcome and files as story:asm-kit-conformance; the 'Through asm_ir/1' section does not retire the old bullets or rewrite Scope"
- file: .engineering/planning/epic/asm-ir-native-backends.md
  line: 59
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:x86-64-writer still promises the System V AMD64 ABI, aggregates passed as the ABI says, callee-saved registers, a libc-checked stack alignment and an assembly runtime (story lines 49, 53, 74-75, 95), which the epic excludes ('a C-ABI entry point') and assigns elsewhere (A3 to A6); the new section replaces only two differences"
- file: .engineering/planning/story/x86-64-writer.md
  line: 88
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "intrinsic.math (math.Sqrt as sqrtsd, roundsd, fprem) is kept in this story, but asm_ir/1 has no fsqrt or math ops (docs/asm-ir.md:102) and the story's Scope owns none of crates/asm-ir, crates/lower or adapters/asm, so no story in the set claims the IR addition and its lowering"
```
