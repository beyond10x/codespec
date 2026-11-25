---
format: aep.planning-md/3
id: review-result:asmir-design-round-1
kind: review-result
status: active
title: 'asm_ir plan: design critic, round 1'
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
revision: 1
---
needs-revision

story:asm-ir-interpreter — its acceptance reads the op corpus that story:asm-x86-64-backend creates (`crates/asm-ir/tests/ops/`), and the lowering and the driver that story:asm-ir-lower-core and story:asm-kit-conformance produce, yet its only `depends_on` edge is to story:asm-ir-model; add `depends_on` to story:asm-x86-64-backend, story:asm-ir-lower-core and story:asm-kit-conformance, or cut the third acceptance line into a later story — .engineering/planning/story/asm-ir-interpreter.md:25-29, .engineering/planning/story/asm-x86-64-backend.md:19-22
story:asm-kit-conformance — it is two things in one item: composing lowering with the x86-64 backend into a first end-to-end writer, and the aarch64 and riscv64 writers plus the cross-check, docs and CI, and its `depends_on story:asm-riscv64-backend` makes the first cir/1-to-running-binary demonstration, and every lowering story behind it (slices, floats, maps), wait for all three backends. Split it so an x86-64 writer and its `conform` part depend only on story:asm-ir-lower-core and story:asm-x86-64-backend, and the remaining writers and cross-check depend on the other backends — aep plan artifact graph (edges story:asm-kit-conformance -> story:asm-riscv64-backend, story:asm-ir-lower-slices-strings-errors -> story:asm-kit-conformance), .engineering/planning/story/asm-kit-conformance.md:53
story:asm-ir-lower-floats — its acceptance "115 in total" is group 6 added to the 111 cases that story:asm-ir-lower-slices-strings-errors brings, but the only edge is to story:asm-kit-conformance, so the two sit in one wave and the total cannot be checked until slices lands; either add `depends_on story:asm-ir-lower-slices-strings-errors` or state the 4 float cases without the cumulative total — .engineering/planning/story/asm-ir-lower-floats.md:25, .engineering/planning/story/asm-ir-lower-slices-strings-errors.md:27
story:asm-ir-lower-maps — its acceptance "122 in total" adds 7 to the 115 that story:asm-ir-lower-floats completes, but it depends only on story:asm-ir-lower-slices-strings-errors; either add `depends_on story:asm-ir-lower-floats` or state the 7 map cases without the cumulative total — .engineering/planning/story/asm-ir-lower-maps.md:25, .engineering/planning/story/asm-ir-lower-floats.md:25
story:asm-kit-conformance — it and the two backend stories both claim the CI install of `qemu-user` and both edit `scripts/ci-tools.sh` (story:asm-aarch64-backend "CI installs it", story:asm-riscv64-backend "CI installs it", this story "CI installs `qemu-user` for AArch64 and RISC-V 64"), so one outcome is claimed by three items; say in this story that the install is the backends' and keep only the gate-part and `check / tools` verification here — .engineering/planning/story/asm-kit-conformance.md:43, .engineering/planning/story/asm-aarch64-backend.md:30, .engineering/planning/story/asm-riscv64-backend.md:31

What I read: 11 artifacts (the epic and 10 stories) via `aep plan artifact show`, plus `aep plan artifact relations`, `graph` (walked 36 edges including those to artifacts outside the set; no cycle in `depends_on`), `validate` (valid), and `docs/asm-ir.md` §§ 5, 8 and 10. I traced about 14 acceptance reads to a producer in the set. The 9 that cross items and need an edge are all recorded: the model reads by lower-core, x86 and interp; the corpus reads by aarch64 and riscv; the lowering and backend reads by kit-conformance. The 5 that cross items and are not recorded are the findings above (interpreter x3, floats total, maps total).

Not a finding, stated as a trade-off: the chain x86-64 -> aarch64 -> riscv64 -> kit-conformance -> slices -> maps is long, but each backend edge has its shared-file reason written in the Scope section (`adapters/asm`, `adapters/asm-run`, `scripts/check-parts.txt`). The alternative to the order is a separate `src/<arch>.rs` registration point plus a per-arch runner target so the two backends can run beside each other. I did not ask for that.

What I could not establish, and out of my lane:
- story:asm-ir-lower-floats owns `crates/lower` and `crates/asm-ir/tests/ops/` but not the writer manifests in `adapters/asm`, while its acceptance needs the claims to grow (acceptance and scope lanes). It also shares `crates/lower` with story:asm-ir-lower-slices-strings-errors in one wave (parallel-safety lane).
- Whether the float cases use `sprintf` and so need group 4: `docs/asm-ir/cir-semantics.md` § 6 not checked for that, so the floats-on-slices edge is stated as a hidden dependency of the total, not of the lowering.

```findings
- file: .engineering/planning/story/asm-ir-interpreter.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the op corpus that story:asm-x86-64-backend creates, and the lowering and the driver that story:asm-ir-lower-core and story:asm-kit-conformance produce, yet its only depends_on edge is to story:asm-ir-model; add depends_on to those three, or cut the third acceptance line into a later story"
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 53
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it is two things in one item, the first end-to-end x86-64 writer and the aarch64/riscv64 writers with cross-check, docs and CI, and its depends_on to story:asm-riscv64-backend makes the first cir/1-to-running-binary demonstration and every lowering story behind it wait for all three backends; split it so an x86-64 writer depends only on story:asm-ir-lower-core and story:asm-x86-64-backend"
- file: .engineering/planning/story/asm-ir-lower-floats.md
  line: 25
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance '115 in total' adds group 6 to the 111 cases of story:asm-ir-lower-slices-strings-errors but no edge records that order; add depends_on story:asm-ir-lower-slices-strings-errors or state the 4 float cases without the cumulative total"
- file: .engineering/planning/story/asm-ir-lower-maps.md
  line: 25
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance '122 in total' adds 7 to the 115 that story:asm-ir-lower-floats completes but it depends only on story:asm-ir-lower-slices-strings-errors; add depends_on story:asm-ir-lower-floats or state the 7 map cases without the cumulative total"
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 43
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the CI install of qemu-user is claimed by this story and by story:asm-aarch64-backend and story:asm-riscv64-backend, all three editing scripts/ci-tools.sh; say the install is the backends' and keep only the gate-part verification here"
```
