---
format: aep.planning-md/3
id: review-result:rd-design-round-2
kind: review-result
status: active
title: Design critic, reader coverage and low-level readers, round 2
relations:
- reviews: story:reader-check-evaluator-step
- reviews: story:reader-java-kit
- reviews: story:reader-c-kit-conventions
- reviews: story:reader-c-kit-plain
- reviews: epic:low-level-readers
- reviews: story:reader-llvm-ir
- reviews: story:reader-asm
revision: 1
---
needs-revision

story:reader-check-evaluator-step — linking the evaluator into `crates/conformance` creates a crate cycle. `story:eval-core`'s `crates/eval` already depends on `crates/conformance`, so the cycle comes from eval-core itself, not only from `story:eval-conformance-runner`. Cargo refuses a cycle. Proposed change: step 4 runs `codespec-write-cir` + `codespec-run-cir`, looked up in `--adapters` like the Go pair, and the edge changes from `depends_on story:eval-core` to `depends_on story:eval-conformance-runner` — qe-eval-core/crates/eval/Cargo.toml:15, qe-eval-core/crates/eval/src/lib.rs:45, crates/conformance/src/conform.rs:1587-1588, .engineering/planning/story/reader-check-evaluator-step.md:10,21
story:reader-java-kit — Java claims `decl.union` only if `story:reader-check-evaluator-step` has merged, but java-kit runs beside that story in the planned slots and has no `depends_on` edge to it. Java's `decl.union` will then be left unclaimed, and no later story claims it. Proposed change: add `depends_on story:reader-check-evaluator-step` to java-kit and rust-kit, and run evaluator-step alone before both slots — .engineering/planning/story/reader-java-kit.md:31-32, story/reader-rust-kit.md:36, `aep plan artifact graph`
story:reader-c-kit-conventions — the hand-written runner mode only handles integer, `bool`, `const char *` and `void` parameters and results. `cs_slice`, `cs_map` and `cs_any` erase their element types, so "compiles a case that includes it" still gives no way to call or print one. Proposed change: `case.c` carries the writer's `/* codespec: type */` and `/* codespec: func */` lines, and both the reader and the hand-written runner mode read them, so there is one source of types — .engineering/planning/story/reader-c-kit-conventions.md:36-42, adapters/c-run/src/lib.rs:44-60
story:reader-c-kit-conventions — the body does not say where the `c` row of `reader_corpus` gets `codespec_rt.h`. The function receives only the kit path, and the header lives in `adapters/c` (`include_str!`), which the kit archive does not ship. Proposed change: keep a copy in the kit and test that it equals `adapters/c/src/codespec_rt.h` — crates/conformance/src/conform.rs:1598, adapters/c/src/write.rs:60
story:reader-c-kit-plain — `sem.multi_assign.order` is listed as plain, but C has no multi-target assignment. Its only form is the writer's temporaries, which is a convention, and the epic already lists the same feature as having no form in Java. Proposed change: move it to conventions or to the unreachable list, because otherwise it takes one of the only two unclaimed slots — story/reader-c-kit-plain.md:20,28-29, conformance/cases/sem-multi-assign-order/case.go:4, epic/reader-coverage.md (Known gaps, Java)
epic:low-level-readers — the reader check requires `codespec-run-<lang>` and runs it on the source at step 1, yet runners are out of scope, so neither report can be produced. Proposed change: bring runners into scope; each story owns `codespec-run-llvm-ir` or `codespec-run-asm` (pinned clang plus a calls driver, as `c-run` does) — epic/low-level-readers.md:29, crates/conformance/src/conform.rs:1572,1321-1331, docs/conformance.md:140-143
story:reader-llvm-ir — LLVM integer types carry no sign, but `cir/1` needs `signed` on every integer type and refuses an expression with no type. Inferring the sign from instructions like `sdiv` or `icmp slt` is a guess for values that are only moved. Proposed change: build the corpus with `-g`, take the sign from `DIBasicType` encodings, and refuse a function that has none — story/reader-llvm-ir.md:17-19, AGENTS.md:57,63-64
story:reader-asm — no rule says where types come from. A register gives the width but not the sign, and the runner has no signature to build calls from. Proposed change: take the sign from the instruction (`idiv`/`jl`/`movs` against `div`/`jb`/`movz`) and refuse when there is none; the corpus also carries the case's C prototypes for the runner — story/reader-asm.md:19-21, AGENTS.md:63-64

Answers:
1. Falling back to the evaluator is sound for `decl.union`. The Go writer refuses it, and eval-core must print `expected.txt` for every kit case. `stmt.foreign` is correctly left out now (revision 3), because E4 stops the evaluator at any foreign statement it executes. The crate cycle exists today, with eval-core itself (first finding).
2. No pair across the two slots edits the same file, apart from the four declared-shared ones (`CHANGELOG.md`, `AGENTS.md`, `Taskfile.yml`, case directories). Every pair that edits one file sits in one slot: `conform.rs` (evaluator-step, conventions), `adapters/c-read` and `adapters/c-run` (plain, conventions), `Reader.java` and `adapters/jvm` (java-kit, quarkus).
3. The design works once the two conventions gaps above are closed. Everything is now on the right side of the plain/conventions cut except `sem.multi_assign.order`.
4. The low-level stories fit `AGENTS.md`: a Rust crate, no LLVM library, `foreign` plus a refusal. Rebuilding control flow from `-O0` output of structured C is reducible and fits one story each, with the irreducible-function refusal already in llvm-ir. The real gaps are the runners and the types.

What I read: 12 artifacts plus eval-core, eval-conformance-runner and both round-1 results (`show`, `relations`, `graph`, `validate`: valid). I walked 22 edges in the set and 6 outside it and found no cycle in the plan graph. I traced 9 acceptance reads to a producer; edges record 7, and the 2 not recorded are rust-kit and java-kit reading evaluator-step.

What I could not establish:
- The store changed while I read it (evaluator-step went to revision 3, rust-kit, java-kit and conventions to revision 4, epic to 4). Findings are against those revisions.
- Outside my lane (parallel safety): llvm-ir and asm edit `CORPUS_LANGS` and `reader_corpus` in `conform.rs` with no edge to evaluator-step or conventions. java-kit lists `examples/ledger/**` as its own, but the shared bullet says the coordinator regenerates it.

```findings
[{"file":".engineering/planning/story/reader-check-evaluator-step.md","line":21,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"linking the evaluator into crates/conformance is a crate cycle because eval-core's crates/eval already depends on crates/conformance; run codespec-write-cir/codespec-run-cir as programs and depend on story:eval-conformance-runner instead of story:eval-core"},
{"file":".engineering/planning/story/reader-java-kit.md","line":31,"category":"design","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"java-kit claims decl.union only if story:reader-check-evaluator-step has merged, but it runs beside that story with no depends_on edge, so no story claims Java decl.union afterwards; add the edge on java-kit and rust-kit"},
{"file":".engineering/planning/story/reader-c-kit-conventions.md","line":40,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"the hand-written runner mode handles only integer, bool, const char * and void, and cs_slice/cs_map/cs_any erase element types; case.c should carry the writer's codespec type and func lines, read by both the reader and the runner"},
{"file":"crates/conformance/src/conform.rs","line":1598,"category":"design","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"story:reader-c-kit-conventions does not name where reader_corpus, which receives only the kit path, gets codespec_rt.h, which lives in adapters/c; keep a copy in the kit and test it equals the writer's"},
{"file":".engineering/planning/story/reader-c-kit-plain.md","line":20,"category":"design","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"sem.multi_assign.order has no direct C form, only the writer's temporaries convention; move it to conventions or to the unreachable list"},
{"file":".engineering/planning/epic/low-level-readers.md","line":29,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"the reader check requires and runs codespec-run-<lang> at step 1, yet runners are out of scope, so neither report can exist; each story should own its runner"},
{"file":".engineering/planning/story/reader-llvm-ir.md","line":17,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"LLVM integer types carry no sign but cir/1 requires signed on every int type; build the corpus with -g, read the sign from DIBasicType encodings, and refuse a function without them"},
{"file":".engineering/planning/story/reader-asm.md","line":19,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"no rule says where types come from: registers give width but not sign, and the runner has no signature; take the sign from the instruction or refuse, and carry the C prototypes in the corpus"}]
```
