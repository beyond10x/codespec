---
format: aep.planning-md/3
id: review-result:asmir-acceptance-round-1
kind: review-result
status: active
title: 'asm_ir plan: acceptance critic, round 1'
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
- reviews: epic:asm-ir-native-backends
revision: 1
---
needs-revision

story:asm-x86-64-backend — "Every corpus module prints its expected output on x86-64, including: ..." names no count and never says each of the 41 operations has a module, so it is true of a corpus that omits an operation — .engineering/planning/story/asm-x86-64-backend.md:40
story:asm-x86-64-backend — the Outcome promises a first line `# codespec: arch x86-64 asm_ir/1` and whole-module refusal of a `features` entry the backend does not claim, and no Done When bullet checks either, so both can be missing at "done" — .engineering/planning/story/asm-x86-64-backend.md:20
story:asm-aarch64-backend — "A host without `qemu-aarch64` skips the run tests with the reason printed" lets the "op corpus prints its expected output on AArch64" bullet pass with nothing run, and no bullet says a skipped run is not a pass in the gate — .engineering/planning/story/asm-aarch64-backend.md:29
story:asm-riscv64-backend — "A host without `qemu-riscv64` skips the run tests with the reason printed" lets "The op corpus prints its expected output on RISC-V 64" pass with nothing run, and no bullet says a skipped run is not a pass in the gate — .engineering/planning/story/asm-riscv64-backend.md:31
story:asm-kit-conformance — "CI installs `qemu-user` for AArch64 and RISC-V 64 in the `check / tools` job" is the same outcome the aarch64 and riscv64 stories claim ("CI installs it", `scripts/ci-tools.sh` in their Scope), so it is already true when those two are done and this story's own work cannot be told from theirs — .engineering/planning/story/asm-kit-conformance.md:43 (cf. asm-aarch64-backend.md:30, asm-riscv64-backend.md:31)
story:asm-kit-conformance — the Outcome states "Exit 0 printed, 2 refused, 1 failed" and the `--asm-ir` and `--describe` modes, but all 78 group-1 cases print, so no Done When bullet makes the refused or failed path or `--asm-ir` observable — .engineering/planning/story/asm-kit-conformance.md:27
story:asm-ir-lower-floats — "The op corpus gains ... printed byte-equal to Go's `strconv.FormatFloat(x, 'g', -1, bits)`" does not say which function prints (the float printer is a lowering runtime function, while the corpus modules "need nothing from the lowering") nor whether the Go bytes are checked in or computed at test time, so two runs of the check can differ — .engineering/planning/story/asm-ir-lower-floats.md:26
story:asm-ir-interpreter — "Every kit case the writers claim" and "Every module of the op corpus" name sets that other stories produce (`story:asm-kit-conformance`, `story:asm-x86-64-backend`, neither a `depends_on`) and that grow with later stories, so the bullets fix no set to check when this story is done — .engineering/planning/story/asm-ir-interpreter.md:28
epic:asm-ir-native-backends — "The stories of groups 2 to 7 and the interpreter are drafted under this epic" is true now, so it reads the same before and after the work, and "each moves the count of passing cases" is false of the interpreter, whose acceptance states no count — .engineering/planning/epic/asm-ir-native-backends.md:53

What I read: 11 of 11 ids (the epic and its 10 stories), each in full with `aep plan artifact show <id>`; `aep plan artifact kinds` and `aep plan artifact lifecycle story|epic`; docs/asm-ir.md §§ 5, 8, 10 and docs/asm-ir/cir-semantics.md § 6 for the 78/111/115/122 counts (they match); `conformance/cases` for `op-add`, `type-slice`, `type-float-compare` and `type-map` (all exist); `/usr/bin/qemu-aarch64` and `/usr/bin/qemu-riscv64` exist.

What I could not establish: whether the gate fails on a skipped run test (no gate step to read yet, so the two skip findings assume it does not); whether `crates/lower/tests/group1.txt` can be generated as `story:asm-ir-lower-core` says (the file does not exist yet). Out of lane, not setting the verdict: `crates/asm-ir/tests/ops/` is owned by `story:asm-x86-64-backend` and `story:asm-ir-lower-floats`, and the aarch64 and riscv64 stories add corpus modules without owning it; `adapters/asm-run` and `scripts/ci-tools.sh` are written by several stories (shared surfaces); `story:asm-ir-interpreter` depends only on `story:asm-ir-model` while its acceptance needs the lowering and the corpus (coupling).

```findings
- file: .engineering/planning/story/asm-x86-64-backend.md
  line: 40
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"Every corpus module prints its expected output on x86-64, including: ...\" names no count and never says each of the 41 operations has a module, so it is true of a corpus that omits an operation"
- file: .engineering/planning/story/asm-x86-64-backend.md
  line: 20
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises a first line `# codespec: arch x86-64 asm_ir/1` and whole-module refusal of an unclaimed `features` entry, and no Done When bullet checks either"
- file: .engineering/planning/story/asm-aarch64-backend.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a host without qemu-aarch64 skips the run tests, so the bullet that the op corpus prints its expected output on AArch64 passes with nothing run, and no bullet says a skipped run is not a pass in the gate"
- file: .engineering/planning/story/asm-riscv64-backend.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a host without qemu-riscv64 skips the run tests, so the bullet that the op corpus prints its expected output on RISC-V 64 passes with nothing run, and no bullet says a skipped run is not a pass in the gate"
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 43
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"CI installs `qemu-user` for AArch64 and RISC-V 64\" is the same outcome the aarch64 and riscv64 stories claim, so it is already true when those two are done and this story's own work cannot be told from theirs"
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome states exit 0 printed, 2 refused, 1 failed and the `--asm-ir` and `--describe` modes, but all 78 group-1 cases print, so no Done When bullet makes the refused or failed path or `--asm-ir` observable"
- file: .engineering/planning/story/asm-ir-lower-floats.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the corpus bullet printed byte-equal to Go's FormatFloat does not say which function prints (the float printer is a lowering runtime function, while corpus modules need nothing from the lowering) nor whether the Go bytes are checked in or computed at test time"
- file: .engineering/planning/story/asm-ir-interpreter.md
  line: 28
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"Every kit case the writers claim\" and \"Every module of the op corpus\" name sets that other stories produce and that grow with later stories, so the bullets fix no set to check when this story is done"
- file: .engineering/planning/epic/asm-ir-native-backends.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"The stories of groups 2 to 7 and the interpreter are drafted under this epic\" is true now, so it reads the same before and after the work, and \"each moves the count of passing cases\" is false of the interpreter, whose acceptance states no count"
```
