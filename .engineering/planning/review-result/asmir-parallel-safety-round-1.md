---
format: aep.planning-md/3
id: review-result:asmir-parallel-safety-round-1
kind: review-result
status: active
title: 'asm_ir plan: parallel-safety critic, round 1'
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

story:asm-x86-64-backend — it and story:asm-ir-lower-core both own root `Cargo.toml` `members`, `Cargo.lock`, `scripts/check-parts.txt` and `AGENTS.md` (cited, both Scope sections), and the x86 body says "it runs beside `story:asm-ir-lower-core`" without admitting the shared files; the root `members` is an explicit list, not a glob, so each story must edit the same line. Remedies, not chosen: add an ordering edge that records the shared files as its reason, or split the surface so neither owns them (for example, one story adds both crates' `members`, parts and rows). The same two stories also both sit behind story:asm-ir-model, which owns these files, so that pair is ordered already — .engineering/planning/story/asm-x86-64-backend.md:56-59, .engineering/planning/story/asm-ir-lower-core.md:49-51
story:asm-ir-interpreter — its acceptance runs the op corpus `crates/asm-ir/tests/ops/` that story:asm-x86-64-backend creates, and every kit case lowered by story:asm-ir-lower-core and claimed by the writers of story:asm-kit-conformance, and its only `depends_on` edge is story:asm-ir-model, so a wave would hold it beside all three producers — .engineering/planning/story/asm-ir-interpreter.md:25-28, .engineering/planning/story/asm-x86-64-backend.md:46-48 (the missing edges are to story:asm-x86-64-backend, story:asm-ir-lower-core and story:asm-kit-conformance, or split the story so the corpus and kit-case runs are separate)
story:asm-ir-lower-floats — it and story:asm-ir-lower-slices-strings-errors both own `crates/lower` (cited), and both depend only on story:asm-kit-conformance, with no edge between them; story:asm-ir-lower-maps also owns `crates/lower` and has no edge to floats. Neither body names the others, and each adds runtime functions to the same crate (the file level inside `crates/lower` is inferred, not cited). Remedies, not chosen: an ordering edge between them, or split `crates/lower` into per-group modules that each story owns — .engineering/planning/story/asm-ir-lower-floats.md:32, .engineering/planning/story/asm-ir-lower-slices-strings-errors.md:35, .engineering/planning/story/asm-ir-lower-maps.md:30
story:asm-ir-lower-maps — its acceptance "122 in total" counts the 4 float cases of story:asm-ir-lower-floats, whose acceptance in turn counts the 111 of story:asm-ir-lower-slices-strings-errors, but it depends only on story:asm-ir-lower-slices-strings-errors, so one wave would hold maps beside floats and the total cannot be checked. The missing edge runs from maps to story:asm-ir-lower-floats; the floats→slices count is already ordered through story:asm-kit-conformance only if an edge floats→slices is added or the totals are restated per group — .engineering/planning/story/asm-ir-lower-maps.md:25, .engineering/planning/story/asm-ir-lower-floats.md:25

The intended first pair does not touch crates, only the shared workspace files in the first finding. Its source files are disjoint. story:asm-ir-lower-core owns `crates/lower` and `crates/cli`. story:asm-x86-64-backend owns `adapters/asm`, `adapters/asm-run` and `crates/asm-ir/tests/ops/`. Both depend on story:asm-ir-model, and neither writes the other's crate. Once the shared files are named or ordered, the pair is safe.

What I read: 10 artifacts (`aep plan artifact show` for each story, `aep plan artifact show epic:asm-ir-native-backends`, `aep plan artifact graph`, `aep plan artifact waves`), plus `Cargo.toml`, `scripts/check-parts.txt` and `docs/asm-ir/` in the tree. Placement: 10 of 10 cited, 0 inferred, 0 unplaced; the only inferred detail is the file inside `crates/lower`. Acceptance reads traced to a producer in the set: 17. Of those, 12 are recorded by a `depends_on` edge (7 directly, 5 only through a chain: riscv64→x86 corpus, conformance→x86 runner, floats→x86 float ops, and two others), and 5 are not recorded (interpreter→x86 corpus, interpreter→lower-core, interpreter→conformance claims, floats→slices count, maps→floats count).

What I could not establish:
- Whether root `Cargo.lock` and `AGENTS.md` collisions matter in practice, since both are generated or table-row edits. That is a person's call.
- `crates/cli` is owned by story:asm-ir-model, story:asm-ir-lower-core and story:asm-kit-conformance, and `scripts/check-parts.txt` by every backend story. They are ordered by `depends_on` edges, so I made no finding.
- `aep plan artifact waves` printed collisions with story:x86-64-writer and story:wasm-writer, which are outside the set, so I did not assess them.
- Out of my lane, not counted toward the verdict:
  - whether `crates/asm-ir/tests/ops/` belongs to story:asm-x86-64-backend rather than to the crate's owner (design critic);
  - story:asm-ir-lower-floats and story:asm-ir-lower-maps count cases that they only claim in manifests their Scope does not list (scope and acceptance critics).

```findings
- file: .engineering/planning/story/asm-x86-64-backend.md
  line: 56
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and story:asm-ir-lower-core both own root Cargo.toml members, Cargo.lock, scripts/check-parts.txt and AGENTS.md (cited in both Scope sections) and the body says it runs beside story:asm-ir-lower-core without naming the shared files; add an ordering edge that records them as its reason, or split the surface so neither owns them"
- file: .engineering/planning/story/asm-ir-interpreter.md
  line: 25
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs the op corpus created by story:asm-x86-64-backend and kit cases lowered by story:asm-ir-lower-core and claimed by story:asm-kit-conformance, and its only depends_on edge is story:asm-ir-model; add the missing edges or merge/split the story"
- file: .engineering/planning/story/asm-ir-lower-floats.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and story:asm-ir-lower-slices-strings-errors both own crates/lower and both depend only on story:asm-kit-conformance with no edge between them, and story:asm-ir-lower-maps also owns crates/lower with no edge to floats; neither body admits it (the file inside crates/lower is inferred); add an ordering edge or split crates/lower per group"
- file: .engineering/planning/story/asm-ir-lower-maps.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its '122 in total' counts the 4 float cases of story:asm-ir-lower-floats (whose 115 counts the 111 of story:asm-ir-lower-slices-strings-errors) but it depends only on slices, so the total is not checkable beside floats; add the edge to story:asm-ir-lower-floats or restate counts per group"
```
