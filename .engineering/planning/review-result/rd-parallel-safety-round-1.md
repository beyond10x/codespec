---
format: aep.planning-md/3
id: review-result:rd-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, reader coverage, round 1
relations:
- reviews: story:reader-go-kit
- reviews: story:reader-rust-kit
- reviews: story:reader-java-kit
- reviews: story:reader-c-kit-conventions
revision: 1
---
needs-revision

story:reader-go-kit — The Scope allows `case.go` edits only and the shared bullet forbids touching `case.cir.json`, but this story cannot land without rewriting kit modules that the other slot and the in-flight eval-core step both read. Any change to `codespec-read-go`'s output, or any `case.go` edit, makes `task conformance` regenerate `case.cir.json` and `expected.txt`, and `features.json` must then follow by hand. Four of its 13 subjects (`decl.union`, `expr.zero`, `intrinsic.sorted_keys`, `type.unit`) are cases whose module is written by hand, and the case check refuses those once the Go reader reaches the subject, so they must become modules the Go reader produces. Both remedies: (a) name in Own `conformance/cases/*/{case.cir.json,expected.txt,features.json}`, `crates/conformance/src/lib.rs` (§ Cases written by hand) and the cross-slot files those modules feed, with an ordering edge that gives them as its reason; or (b) leave existing `case.cir.json` untouched and add variant cases instead (the variant-case finding below applies). Cited — crates/conformance/src/lib.rs:13, :35-48, :1238-1248; Taskfile.yml:414-418, :446-452; adapters/rust-run/tests/acceptance.rs:3, :53.

story:reader-c-kit-conventions — Its round-trip acceptance reads the C writer's output of every kit `case.cir.json`, and story:reader-go-kit rewrites those modules (finding above). The two are in different slots and no `depends_on` edge runs between them. Add the edge, or state that the round-trip count is taken on the merged head after go-kit. Cited — `aep plan artifact show story:reader-c-kit-conventions` (Acceptance, bullet 2); crates/conformance/src/lib.rs:13.

story:reader-java-kit — The shared Scope bullet does not say whether a unit may add a new case directory, yet variant cases are the kit's documented way to give a language a reachable subject, and Java added `expr-zero-error` and `type-unit-method` that way. A new case breaks the "of the 135 cases" counts in the Rust and Java acceptances. It also adds one more module that eval-core's step, the rust-run acceptance test and every writer report must pass. In the shared bullet, either forbid adding cases, or have the unit name the case and phrase the counts as "of the kit's cases on the merged head". Cited — crates/conformance/src/lib.rs:69-73; adapters/java/README.md:33-34; docs/evaluator.md:38, :122-123.

story:reader-rust-kit — Inferred (holds only if a reader's output on the example changes). Neither this Scope, go-kit's, nor the coordinator-regenerated list names `examples/**`, `task examples` or `website/docs/examples/*.md`. Yet `examples/series/series.rust-read.cir.json` and `out/go-from-rust/` come from the Rust reader reading `out/rust`, which comes from the Go reader's `series.read.cir.json`, so go-kit and rust-kit both change them and `task check` compares them. Other example files the readers rewrite: Rust reader → `examples/inventory/*`; Go reader → orders, greeter, envcheck, `examples/netcheck/*`. Add `examples/*/*.cir.json`, `examples/*/out/**`, `examples/netcheck/*` and `website/docs/examples/*.md` to the coordinator list (`task examples`, `task examples-netcheck`, then `task docs-generate` on the merged head), or add an ordering edge between go-kit and rust-kit. Citations: Taskfile.yml:281-292, :556-581; website/docs/examples/series.md:804-822; AGENTS.md:127.

story:reader-java-kit — Same slot only, so not a collision. The Scope does not name the checked-in files that a change to `adapters/java` or `adapters/jvm` rewrites: `examples/ledger/**`, `examples/fees/**` and `adapters/groovy/testdata/lending.read.cir.json` (compared byte-for-byte in the gate). The same gap applies to story:reader-java-quarkus-refusals. Cited — Taskfile.yml:179, :293-301.

What you asked about specifically:
- **eval-core:** new `rust/`, `java/`, `c/` or `groovy/` sources cannot break its step, because the evaluator reads only `case.cir.json`, `calls.json` and `expected.txt` (docs/evaluator.md:122). go-kit's `case.go` edits and Go reader changes can break it, through the regenerated `case.cir.json`. Any new case can too.
- **Hard-coded counts:** none in tests. Searching for 135, 127, 114, 89, 78, 77 and 59 found only prose (docs/evaluator.md:38).
- **Kit digest:** it hashes all of `cases/` (conform.rs:1059), so every unit's new source changes every report, other-slot ones included. The coordinator-regenerated list already covers `website/data/conformance/*.json`, so that is handled as drafted.
- **Other shared surfaces:** none of the seven edits `CORPUS_LANGS`, `crates/docs` (floor only), the Go runner, or `adapters/c-run` / `adapters/rust-run` across slots. `CHANGELOG.md`, `AGENTS.md`, `Taskfile.yml` and `conformance/cases/<case>/` are declared shared and left to the coordinator.

What I read: 8 artifacts (`aep plan artifact show` for each, plus story:eval-core). In the tree: Taskfile.yml, crates/conformance/src/{lib.rs,conform.rs}, crates/docs/src/main.rs, adapters/rust-run/tests/acceptance.rs, adapters/c-run/tests/acceptance.rs, docs/evaluator.md, AGENTS.md § Generated files, and `git grep` for the counts and the case names. Surfaces: 7 stories cited, 0 inferred, 0 could not be placed (the epic is not a work unit). Acceptance reads traced to a producer in the set: 7. A `depends_on` edge records 6 of them; c-kit-conventions → go-kit is the missing one. The case-count reads also depend on a producer if a case gets added.

What I could not establish:
- Whether go-kit will lift `type.unit` or `expr.zero` across all cases or only in its subject cases. Kit-wide lifting would widen the first finding to most `case.cir.json` files.
- Whether the in-flight evaluator handles every node that reader-produced union, zero, sorted-keys and unit cases contain.
- For coverage-floor, the floor file's path and "one site page" are unnamed. The page is probably `website/docs/reference/support.mdx` (inferred). It runs alone, so this is not a collision.
- Out of my lane (acceptance): `task adapters` only builds, and the adapter tests run in `adapters-check`.

```findings
- file: crates/conformance/src/lib.rs
  line: 1238
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Scope allows `case.go` edits only and the shared bullet forbids touching `case.cir.json`, but this story cannot land without rewriting kit modules that the other slot and the in-flight eval-core step both read"
- file: "aep plan artifact show story:reader-c-kit-conventions"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its round-trip acceptance reads the C writer's output of every kit case.cir.json, which story:reader-go-kit rewrites in the other slot, and no depends_on edge orders them"
- file: crates/conformance/src/lib.rs
  line: 69
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the shared Scope bullet does not say whether a unit may add a new case directory, yet a variant case changes the 135-case counts of the Rust and Java acceptances and adds a module the eval-core step must pass"
- file: Taskfile.yml
  line: 289
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred: neither this Scope, go-kit's, nor the coordinator-regenerated list names examples/** or website/docs/examples/*.md, yet go-kit and rust-kit both change examples/series/series.rust-read.cir.json and out/go-from-rust, which task check compares"
- file: Taskfile.yml
  line: 179
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Scope does not name examples/ledger/**, examples/fees/** and adapters/groovy/testdata/lending.read.cir.json, which a change to adapters/java or adapters/jvm rewrites"
```
