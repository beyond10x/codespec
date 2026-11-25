---
format: aep.planning-md/3
id: review-result:asmir-design-round-2
kind: review-result
status: active
title: 'asm_ir plan: design critic, round 2'
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
revision: 1
---
needs-revision

All five round-1 findings are fixed. I found two new ones.

Round-1 status:
1. story:asm-ir-interpreter now depends on story:asm-x86-64-writer, which depends on lower-core and the backend. The corpus, the lowering and the driver are all reachable by recorded edges.
2. story:asm-kit-conformance is split. story:asm-x86-64-writer depends only on lower-core and the x86-64 backend. Slices, floats, maps and the interpreter now hang off the x86-64 writer, not off riscv64.
3. story:asm-ir-lower-floats now depends on story:asm-ir-lower-slices-strings-errors.
4. story:asm-ir-lower-maps now depends on story:asm-ir-lower-floats.
5. The qemu CI install is now the two backend stories' alone. story:asm-kit-conformance says "the emulators are installed by the backend stories" (asm-kit-conformance.md, Done When).

Findings:

story:asm-ir-lower-slices-strings-errors — its outcome says "The writers' manifests claim the features of these groups" and its scope owns "the manifest of `adapters/asm` (shared by the three writers)". The aarch64 and riscv64 writer programs and manifests are created by story:asm-kit-conformance, and no edge orders the two. Landing first, the claim cannot be made. Landing after, it is made against a manifest whose acceptance only runs x86-64. Either state "the x86-64 writer's manifest" and leave the other two writers' claims to the cross-check's intersection (docs/asm-ir.md:205), or add `depends_on story:asm-kit-conformance` and a conform line for those targets — .engineering/planning/story/asm-ir-lower-slices-strings-errors.md:27,40; .engineering/planning/story/asm-kit-conformance.md:20-21 (Done When: the two writers conform with 78 cases of group 1 only)

story:asm-ir-lower-floats — the same seam: "The writers' manifests claim the float features", with no edge to story:asm-kit-conformance and only an x86-64 acceptance — .engineering/planning/story/asm-ir-lower-floats.md:26,41

story:asm-ir-lower-maps — the same seam: "The writers' manifests claim the map features", with no edge to story:asm-kit-conformance and only an x86-64 acceptance — .engineering/planning/story/asm-ir-lower-maps.md:25,35

story:x86-64-writer — it adds four operations to `asm_ir::Op` in `crates/asm-ir`. Two siblings read that enum: story:asm-x86-64-backend's test "fails when an operation of `asm_ir::Op` has no module" over the 41-operation corpus, and story:asm-ir-interpreter's `interp.rs`, which has to execute every operation. This story's scope names neither `crates/asm-ir/tests/ops` nor the interpreter, and its only edges into the epic are to kit-conformance and floats. Add `depends_on story:asm-ir-interpreter` and own the four corpus modules and interpreter arms, or say the completeness test skips feature-guarded operations — .engineering/planning/story/x86-64-writer.md:72-76 (Done When), .engineering/planning/story/asm-x86-64-backend.md:52-53, .engineering/planning/story/asm-ir-interpreter.md:25-27

What I read: 13 artifacts via `aep plan artifact show`. They are the epic, its 11 stories, story:x86-64-writer and review-result:asmir-design-round-1. I also ran `relations`, `graph` and `validate`, and read docs/asm-ir.md §§ 4, 7-10.
- **Graph:** I walked about 60 edges, including edges outside the set to story:wasm-writer, story:native-run-harness, story:native-speed-comparison, story:reader-asm and epic:ecmascript-to-native. There is no cycle in `depends_on`. Nothing in the epic depends on story:x86-64-writer.
- **Acceptance reads:** about 18 cross-item reads traced to a producer in the set. 16 are recorded by a direct or transitive `depends_on`. The two that are not recorded are the manifest claims and the four-operation corpus completeness above.
- **Validator:** `validate` says valid. It reports only stale whole-repo review outcomes, which are not mine.

Not findings, stated as trade-offs:
- The chain x86-64 writer -> aarch64 -> riscv64 -> kit-conformance has its shared-file reason written in each Scope section (`adapters/asm`, `adapters/asm-run`, `scripts/ci-tools.sh`). The alternative is a separate registration point per architecture, so the two backends could run side by side. I did not ask for it.
- story:x86-64-writer waits on story:asm-kit-conformance, and so on aarch64 and riscv64, for the shared `adapters/asm` and `adapters/asm-run`. The cost is that the ECMAScript bench target sits behind both other backends. The alternative is to order it before story:asm-aarch64-backend instead. The reason is written in its Scope, so I did not ask for the edge to be removed.

What I could not establish, and out of my lane:
- I could not tell whether the four new operations of story:x86-64-writer also drift the checked-in `schemas/asm-ir-1.schema.json` and the printer fixture. Both belong to story:asm-ir-model, and neither is in story:x86-64-writer's scope. That is a scope and parallel-safety matter.
- Whether the shared `crates/lower` and `adapters/asm` manifest in slices, floats and maps can run at once is parallel-safety. Here it is ordered by edges.
- The 78, 111, 115 and 122 counts and the acceptance wording are the acceptance lane.

```findings
- file: .engineering/planning/story/asm-ir-lower-slices-strings-errors.md
  line: 27
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome says the writers' manifests claim the features of these groups and its scope owns the manifest shared by the three writers, but the aarch64 and riscv64 writer programs are created by story:asm-kit-conformance with no edge between them and the acceptance only runs x86-64; say the x86-64 manifest only (the cross-check takes the intersection) or add depends_on story:asm-kit-conformance with a conform line for those targets"
- file: .engineering/planning/story/asm-ir-lower-floats.md
  line: 26
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome says the writers' manifests claim the float features, but the aarch64 and riscv64 writers are created by story:asm-kit-conformance, no edge orders the two and the acceptance only runs x86-64; say the x86-64 manifest only or add depends_on story:asm-kit-conformance with a conform line for those targets"
- file: .engineering/planning/story/asm-ir-lower-maps.md
  line: 25
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome says the writers' manifests claim the map features, but the aarch64 and riscv64 writers are created by story:asm-kit-conformance, no edge orders the two and the acceptance only runs x86-64; say the x86-64 manifest only or add depends_on story:asm-kit-conformance with a conform line for those targets"
- file: .engineering/planning/story/x86-64-writer.md
  line: 72
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it adds four operations to asm_ir::Op that story:asm-x86-64-backend's every-operation corpus test and story:asm-ir-interpreter's interp.rs both read, and neither its scope nor its acceptance nor any edge names them; add depends_on story:asm-ir-interpreter and own the four corpus modules and interpreter arms, or state that the completeness test skips feature-guarded operations"
```
