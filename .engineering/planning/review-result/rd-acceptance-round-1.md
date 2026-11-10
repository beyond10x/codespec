---
format: aep.planning-md/3
id: review-result:rd-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, reader coverage, round 1
relations:
- reviews: epic:reader-coverage
- reviews: story:reader-go-kit
- reviews: story:reader-rust-kit
- reviews: story:reader-java-kit
- reviews: story:reader-java-quarkus-refusals
- reviews: story:reader-c-kit-plain
- reviews: story:reader-c-kit-conventions
revision: 1
---
needs-revision

epic:reader-coverage — No reader can ever reach the Done When floors (Go 124, Rust 121, Java 121, C 115). Readers count only claimed features, and the manifest refuses a reader claiming any of the 7 `fact.*` features. The Go writer refuses `decl.union` at step 4, which caps every reader at 119. It also refuses a non-Go `stmt.foreign` and writes extern paths as Go imports, so `expr.extern`, `expr.extern_method` and `type.extern` cannot run; that caps Rust, Java and C at 115. Reword to "Go 119 (its ceiling), Rust ≥113, Java ≥113, C ≥110 of the ceiling 115, each `fail` 0, `conforms: true`" — .engineering/planning/epic/reader-coverage.md:28; crates/model/src/manifest.rs:23,74; adapters/go/cmd/codespec-write-go/write.go:364,870-871,1015-1018; website/data/conformance/writer-go.json (`decl.union` refused)
epic:reader-coverage — "at most 2 refusals" goes against the README: the 8 quickstart throws "stay refused and `foreign`: `cir/1` has no statement that leaves a function by an exception", and the epic keeps `cir/1` unchanged. Reword to "at most 10 refusals, 8 of them the documented jar throws" — .engineering/planning/epic/reader-coverage.md:31; adapters/java/README.md:103-106,125
story:reader-go-kit — 8 of the 13 listed features can never pass: the 7 `fact.*` (no reader may claim them) and `decl.union` (refused at step 4). So "at least 124" and "At most 3 stay unclaimed" are unreachable. Reword to "`summary.pass` 119 of 127; the 8 unclaimed are exactly the seven `fact.*` and `decl.union`, each documented" — .engineering/planning/story/reader-go-kit.md:16,24,28; manifest.rs:23; write.go:364
story:reader-rust-kit — The ceiling is 115, not 121. The 38 untested features include 7 `fact.*`, `decl.union`, `stmt.foreign`, `expr.extern`, `expr.extern_method` and `type.extern`, and none of these can pass. `unop.plus` has no Rust source form, which makes the reachable maximum 114. Reword to "at least 113 of 127, at most 14 unclaimed, those 12 plus `unop.plus` named with their reason" — .engineering/planning/story/reader-rust-kit.md:24,28; write.go:870-871,1015-1018; website/data/conformance/reader-rust.json
story:reader-java-kit — The same 12 features can never pass, so 121 cannot be reached (ceiling 115) and "At most 6 stay unclaimed" is contradicted. Reword to "at least 113 of 127, at most 14 unclaimed, the 12 named in the README as unclaimable by step 3/4 or the manifest" — .engineering/planning/story/reader-java-kit.md:24,26; manifest.rs:23; write.go:364,870-871,1015-1018
story:reader-java-quarkus-refusals — "at most 2 refusals" cannot be met. The README says the 8 jar throws stay refused and `foreign` because `cir/1` cannot express them, and this story keeps `cir/1` unchanged (Scope). Reword to "at most 10 refusals, of which 8 are the jar throws README § A quickstart documents" and drop the jar throw from the Outcome list — .engineering/planning/story/reader-java-quarkus-refusals.md:18,26; adapters/java/README.md:103-106,125
story:reader-java-quarkus-refusals — "in a form `codespec effects` or a later query can read" cannot be checked, because a later query does not exist. Reword to "`codespec effects` output over `adapters/java/testdata/quarkus` names each persistence annotation, asserted by a reader test" — .engineering/planning/story/reader-java-quarkus-refusals.md:32; Taskfile.yml:223-236
story:reader-c-kit-plain — 4 of the 30 listed features can never pass (`decl.union`, `stmt.foreign`, `expr.extern`, `type.extern`). So "At most 2 of the 30 stay unclaimed" and "at least 87" (59+28) are unreachable; the maximum is 85. Drop those four from the list and reword to "at least 84 of 127, at most 1 of the 26 unclaimed" — .engineering/planning/story/reader-c-kit-plain.md:19-22,29,31; write.go:364,870-871,1015-1018
story:reader-c-kit-conventions — The list asks to claim "the seven `fact.*` features", which the manifest refuses to any reader, and `expr.extern_method`, which cannot run at step 4. With the 4 from the plain story that makes 12 that can never pass, so "115 / at most 12" requires every other feature to pass, with no slack. Remove `fact.*` and `expr.extern_method` from the list and reword to "at least 110 of 127, at most 17 unclaimed, 12 of them the unclaimable ones" — .engineering/planning/story/reader-c-kit-conventions.md:23,26,31,37; manifest.rs:23
story:reader-c-kit-conventions — "`codespec-write-c` already emits each of them in a fixed C form" is false for 8 listed features. The C writer refuses `decl.interface`, `decl.type_params`, `expr.iface_call`, `expr.instantiate`, `expr.lambda`, `expr.spread`, `expr.to_iface` and `type.param`, so the round-trip bullet has no writer output to check for them. Name, per feature, the C form the reader will recognise, or the story that defines it — .engineering/planning/story/reader-c-kit-conventions.md:18; website/data/conformance/writer-c.json

What I read: all 8 ids, each through `aep plan artifact show <id>`. I also read the module doc and `reader_case` in crates/conformance/src/conform.rs, `features_used` in crates/conformance/src/lib.rs, crates/model/src/manifest.rs, the foreign/extern/union paths in the Go writer's write.go, every reader report and writer report (go, rust, c), adapters/java/README.md:100-136, Taskfile.yml, and AGENTS.md § Generated files.

What I could not establish:
- Whether Java has source forms for `type.chan`, `stmt.send`, `unop.recv`, `decl.embedded`, `type.tuple`, `op.and_not` and `assign.and_not`; any it lacks lowers the Java ceiling further.
- How many of the 15 quickstart functions hold the 8 throws, which decides whether "14 of 15 without `foreign`" can be met. I had no clone, so I could not run `task java-quickstart`.
- `boundary_fail 0` is always true for a reader (the boundary array is empty), so that clause reads the same before and after. It is harmless and I did not count it as a finding.
- story:reader-coverage-floor: no acceptance finding.
- Outside my lane: none.

```findings
- {file: .engineering/planning/epic/reader-coverage.md, line: 28, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "No reader can reach the Done When floors (Go 124, Rust 121, Java 121, C 115): fact.* cannot be claimed by a reader and decl.union is refused at step 4, capping all readers at 119; the Go writer also refuses non-Go stmt.foreign and cannot run foreign externs, capping Rust, Java and C at 115."}
- {file: .engineering/planning/epic/reader-coverage.md, line: 31, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "At most 2 refusals goes against the README: the 8 quickstart throws stay refused and foreign, cir/1 cannot express them, and the epic keeps cir/1 unchanged."}
- {file: .engineering/planning/story/reader-go-kit.md, line: 24, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "8 of the 13 listed features (seven fact.*, decl.union) can never pass, so at least 124 and at most 3 unclaimed are unreachable; the ceiling is 119."}
- {file: .engineering/planning/story/reader-rust-kit.md, line: 24, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "The ceiling is 115, not 121: fact.*, decl.union, stmt.foreign and the three extern features can never pass, and unop.plus has no Rust source form, making the reachable maximum 114."}
- {file: .engineering/planning/story/reader-java-kit.md, line: 24, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "The same 12 features can never pass, so 121 cannot be reached (ceiling 115) and at most 6 unclaimed is contradicted."}
- {file: .engineering/planning/story/reader-java-quarkus-refusals.md, line: 26, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "At most 2 refusals cannot be met: the README says the 8 jar throws stay refused and foreign because cir/1 cannot express them, and this story keeps cir/1 unchanged."}
- {file: .engineering/planning/story/reader-java-quarkus-refusals.md, line: 32, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "In a form codespec effects or a later query can read cannot be checked, because a later query does not exist."}
- {file: .engineering/planning/story/reader-c-kit-plain.md, line: 31, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "4 of the 30 listed features (decl.union, stmt.foreign, expr.extern, type.extern) can never pass, so at most 2 of the 30 unclaimed and at least 87 are unreachable; the maximum is 85."}
- {file: .engineering/planning/story/reader-c-kit-conventions.md, line: 26, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "The list asks to claim the seven fact.* features, which the manifest refuses to any reader, and expr.extern_method, which cannot run at step 4; with the 4 from the plain story, 115 and at most 12 unclaimed require every other feature to pass, with no slack."}
- {file: .engineering/planning/story/reader-c-kit-conventions.md, line: 18, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "codespec-write-c already emits each of them in a fixed C form is false for 8 listed features the C writer refuses, so the round-trip bullet has no writer output to check for them."}
```
