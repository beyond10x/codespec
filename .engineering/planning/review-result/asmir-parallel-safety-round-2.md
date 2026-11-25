---
format: aep.planning-md/3
id: review-result:asmir-parallel-safety-round-2
kind: review-result
status: active
title: 'asm_ir plan: parallel-safety critic, round 2'
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
revision: 1
---
needs-revision

story:asm-x86-64-backend — it says "the two share no file" beside `story:asm-ir-lower-core`, yet both list root `Cargo.lock` in `scope` and in `## Scope`, so `aep plan artifact waves` puts them in waves 2 and 3, not side by side (cited, both bodies). Round-1 finding 1 is only partly fixed: root `Cargo.toml`, `scripts/check-parts.txt` and `AGENTS.md` are gone from both, and `Cargo.lock` remains. Remedies, not chosen: an ordering edge between the two that records `Cargo.lock` as its reason, or drop `Cargo.lock` from one or both `scope` lists (the model story already registers the skeleton crates with their dependency lists). — .engineering/planning/story/asm-x86-64-backend.md:13, .engineering/planning/story/asm-x86-64-backend.md:75-77, .engineering/planning/story/asm-ir-lower-core.md:13, .engineering/planning/story/asm-ir-lower-core.md:57

story:asm-ir-interpreter — its `scope` names the whole `crates/asm-ir/tests`, which contains `crates/asm-ir/tests/ops/`. `story:asm-aarch64-backend` and `story:asm-riscv64-backend` both add modules and `index.txt` lines there, and neither has an edge to it (`waves` prints both as `collision … crates/asm-ir/tests/ops`, cited). The interpreter's acceptance runs "every module listed in `index.txt`", so it reads modules those two stories add and has no `depends_on` edge to them. Remedies, not chosen: narrow its scope to its own test file and state the `index.txt` reads, or add edges that record `tests/ops` as the reason. — .engineering/planning/story/asm-ir-interpreter.md:15, .engineering/planning/story/asm-ir-interpreter.md:31-32, .engineering/planning/story/asm-aarch64-backend.md:47-48, .engineering/planning/story/asm-riscv64-backend.md:48-49

story:asm-ir-lower-slices-strings-errors — its `scope` claims the whole `adapters/asm` for what its body calls "the manifest", while `story:asm-aarch64-backend`, `story:asm-riscv64-backend` and `story:asm-kit-conformance` own `adapters/asm/src/*.rs` and the writer binaries there. `waves` prints each as a collision (cited), with no edge from the lowering stories to them. Narrow the entry to the manifest file, or add the edges. — .engineering/planning/story/asm-ir-lower-slices-strings-errors.md:40, .engineering/planning/story/asm-aarch64-backend.md:47, .engineering/planning/story/asm-kit-conformance.md:59

story:asm-ir-lower-floats — same defect: `adapters/asm` is claimed for "the manifest" and collides with `adapters/asm/src/aarch64.rs`, `adapters/asm/src/riscv64.rs` and the `story:asm-kit-conformance` binaries. Narrow it to the manifest file, or add the edges. — .engineering/planning/story/asm-ir-lower-floats.md:41, .engineering/planning/story/asm-riscv64-backend.md:48

story:asm-ir-lower-maps — same defect: `adapters/asm` is claimed for "the manifest" and collides with the backend and conformance stories. Narrow it to the manifest file, or add the edges. — .engineering/planning/story/asm-ir-lower-maps.md:35, .engineering/planning/story/asm-kit-conformance.md:59

Round-1 status:
- Finding 2 (interpreter edges): fixed. Its `depends_on` is now `story:asm-x86-64-writer`, which covers the corpus, group 1 and the driver. The `tests/ops` collision above is the part that remains.
- Finding 3 (floats and slices both own `crates/lower`): fixed. The edges run slices, then floats, then maps.
- Finding 4 (the "122 in total" count): fixed by the same edges.
- Finding 1: partly fixed, as the first finding says.

What I read: 11 stories, plus the shared-surface text of round 1.
- Commands: `aep plan artifact show` on the epic's stories through `cat` of their files, `aep plan artifact graph`, `aep plan artifact waves` (text and json), `aep plan artifact show review-result:asmir-parallel-safety-round-1`.
- I computed `depends_on` ancestry for the 11 by hand and listed the colliding pairs it leaves unordered.
- Surfaces: 11 of 11 cited, 0 inferred, 0 unplaceable.
- Acceptance reads traced to a producer in the set: 18. Recorded by a `depends_on` edge, direct or by chain: 16. Unrecorded: 2 (interpreter reading AArch64 and RISC-V 64 modules in `index.txt`).
- The intended first pair, `story:asm-ir-lower-core` beside `story:asm-x86-64-backend`, shares only `Cargo.lock`. Their source surfaces are disjoint, and neither acceptance reads state the other creates. `waves` still does not put them in one wave until the `Cargo.lock` claim is resolved.

What I could not establish:
- Whether the shared `Cargo.lock` matters in practice, since it is generated. That is a person's call.
- The file inside `adapters/asm` that holds "the manifest". No body names it, so how much of `adapters/asm` the three lowering stories really touch is inferred.
- `waves` prints collisions with `story:x86-64-writer`, `story:wasm-writer` and many unrelated stories (`crates/cli`, `Cargo.lock`), none of them in the set. I did not assess them.
- Out of my lane (not counted toward the verdict): `story:asm-kit-conformance` has the title "The AArch64 and RISC-V 64 writers…" in its heading and a summary naming three writers. That is for the design and scope critics.

```findings
- file: .engineering/planning/story/asm-x86-64-backend.md
  line: 75
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body says it shares no file with story:asm-ir-lower-core, but both list root Cargo.lock in scope and in their Scope sections (cited), and aep plan artifact waves puts them in waves 2 and 3; add an ordering edge that records Cargo.lock as its reason, or drop Cargo.lock from one or both scope lists"
- file: .engineering/planning/story/asm-ir-interpreter.md
  line: 15
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope names the whole crates/asm-ir/tests, which contains crates/asm-ir/tests/ops that story:asm-aarch64-backend and story:asm-riscv64-backend add modules and index.txt lines to (waves collision, cited), and its acceptance runs every module in index.txt with no depends_on edge to those two; narrow the scope and state the reads, or add edges recording tests/ops as the reason"
- file: .engineering/planning/story/asm-ir-lower-slices-strings-errors.md
  line: 40
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope claims all of adapters/asm for what the body calls the manifest, which collides with story:asm-aarch64-backend, story:asm-riscv64-backend and story:asm-kit-conformance (cited), with no edge; narrow the entry to the manifest file, or add edges"
- file: .engineering/planning/story/asm-ir-lower-floats.md
  line: 41
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope claims all of adapters/asm for the manifest, which collides with the aarch64, riscv64 and kit-conformance stories (cited), with no edge; narrow the entry to the manifest file, or add edges"
- file: .engineering/planning/story/asm-ir-lower-maps.md
  line: 35
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope claims all of adapters/asm for the manifest, which collides with the aarch64, riscv64 and kit-conformance stories (cited), with no edge; narrow the entry to the manifest file, or add edges"
```
