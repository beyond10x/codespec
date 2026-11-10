---
format: aep.planning-md/3
id: review-result:rd-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, reader coverage, round 2
relations:
- reviews: epic:reader-coverage
- reviews: story:reader-rust-kit
- reviews: story:reader-java-kit
- reviews: story:reader-check-evaluator-step
- reviews: story:reader-c-kit-plain
- reviews: story:reader-c-kit-conventions
- reviews: story:reader-java-quarkus-refusals
revision: 1
---
needs-revision

epic:reader-coverage — The Done When floors "Rust at least 112, Java at least 108" are above what the repository shows each reader can reach (Rust 109, Java 107), so this epic can never be closed; reword to "Rust at least 105, Java at least 95" — .engineering/planning/epic/reader-coverage.md:29-30
story:reader-rust-kit — "at least 112 … At most 8 stay unclaimed" cannot be met: the crate doc already says why `stmt.foreign`, `unop.plus`, `unop.recv`, `type.any` and `expr.nil` stay unclaimed, the epic adds the three extern features, and Rust has no variadic function and no embedding, so `decl.variadic`, `expr.spread` and `decl.embedded` cannot pass either; that is 11 unclaimed and a ceiling of 109. Reword to "at least 105 of 120, at most 15 unclaimed, each named with its reason", and claim only `decl.union` once the evaluator story is merged — .engineering/planning/story/reader-rust-kit.md:28,32-33; adapters/rust-read/src/lib.rs:129-137; website/data/conformance/writer-rust.json (decl.embedded, decl.variadic refused, "no Rust form")
story:reader-java-kit — "at least 108 … At most 12" cannot be met: the README's table gives 9 features with no Java source form (`assign.and_not`, `op.and_not`, `decl.newtype`, `sem.multi_assign.order`, `stmt.send`, `type.chan`, `unop.recv`, `stmt.var`, `unop.deref`), and `stmt.foreign` plus the three extern features make 13, so the ceiling is 107. Keeping the README's 7 refusals by design (four shifts, `expr.to_error`, `intrinsic.errorf`, `sem.switch.break`) lowers it to 100. Reword to "at least 95 of 120, at most 25 unclaimed" and drop `stmt.foreign` from the evaluator clause — .engineering/planning/story/reader-java-kit.md:23,26,28; adapters/java/README.md:40-59
story:reader-check-evaluator-step — The Outcome and summary say the fallback lets `stmt.foreign` be judged, but the evaluator stops at every `foreign` statement (E4). The `stmt-foreign` case runs its `defer done()` foreign statement on the only path, so this fallback only ever gives `refused`. Reword to "so `decl.union` can be judged; `stmt.foreign` stays `refused` (evaluator E4)" — .engineering/planning/story/reader-check-evaluator-step.md:7,19; docs/evaluator.md:25-26; conformance/cases/stmt-foreign/case.cir.json
story:reader-c-kit-plain — "`stmt.foreign` is claimed only when `story:reader-check-evaluator-step` is merged" makes a claimed feature that is refused, and conform.rs counts that as `fail`, which breaks "`fail` 0". Drop `stmt.foreign` from the 21 and name it as unclaimed. That leaves 18 of 20 needed for 77, while `decl-variadic` (`path.Join`, a slice), `expr-nil` (a nil slice) and both `expr-zero` cases (errors, strings) need a variant case without strings or slices; say so for each — .engineering/planning/story/reader-c-kit-plain.md:20,26,29; crates/conformance/src/conform.rs:97-99
story:reader-c-kit-conventions — The candidates still include `expr.extern` and `type.extern`. `codespec-write-c` refuses both, so the story's own rule ("reads those forms back, and only those") excludes them, and the epic lists them as gaps for C. Move both to "Not candidates" with `expr.extern_method` — .engineering/planning/story/reader-c-kit-conventions.md:27,32; .engineering/planning/epic/reader-coverage.md:36-37; website/data/conformance/writer-c.json
story:reader-java-quarkus-refusals — The title "at most two refusals" and the summary "jar exceptions … are read instead of refused" contradict the acceptance, which allows at most 10 refusals and keeps the 8 jar throws. Retitle to "at most ten refusals" and drop "and jar exceptions" from the summary — .engineering/planning/story/reader-java-quarkus-refusals.md:6-7

Floors, computed against the 120 features a reader can claim:
- Go 115: reachable. 114 verified today, plus `type.tuple`, which passes unclaimed (`pass_unclaimed` 1). The other five are refused at step 3 because the module does not use them.
- Rust ≥112: unreachable. Ceiling 109, or 110 if the reader learns to type `type.any`.
- Java ≥108: unreachable. Ceiling 107, or 100 if the README's refusals by design stay.
- C ≥77: cannot tell. Ceiling 79 (59 + 21 − `stmt.foreign`). It needs at most two misses among the three cases above that need variants.
- C ≥100: reachable on the count. Ceiling 108 (120 − the 8 the C writer refuses − 3 externs − `stmt.foreign`), so 8 features of slack.
- Quarkus ≤10: reachable on the count. 25 − 17 mapped = 8 jar throws, so 2 of slack.

Round 1:
- Fixed: findings 2, 3, 7, 8 and 10.
- Fixed in part:
  - Finding 1: Go and C are fixed; Rust and Java are not.
  - Findings 4 and 5: the ceilings were lowered, but the floors are still above them.
  - Finding 6: the acceptance is fixed; the title and summary are not.
  - Finding 9: `fact.*` and `expr.extern_method` were removed, but `expr.extern` and `type.extern` are still candidates.

What I read: all 9 ids plus `review-result:rd-acceptance-round-1`, each through `aep plan artifact show`. I also read support-matrix.json (`read` columns), the reader and writer reports, the verdict table in conform.rs, lib.rs § Cases written by hand, the rust-read crate doc, adapters/java/README.md:34-130, docs/evaluator.md §§ 2, 3 and 6, and the case.go files of the kit cases named above.

What I could not establish:
- Whether the 17 Quarkus refusals, once mapped, expose new refusals in bodies read for the first time. I had no quickstart clone.
- How many kit cases `codespec-write-c` writes without refusal, so whether the round-trip count "at least 100" can be met. My estimate is at most 116 of 135.
- Whether Java has a form for `type.tuple`, `decl.embedded` or `stmt.lookup_else`. Each one it lacks lowers its ceiling by one more.
- Outside my lane: the epic summary still says "127", and the Go, Rust and Java summaries still give the old counts (13, 38, 50).

```findings
- {file: .engineering/planning/epic/reader-coverage.md, line: 29, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "The Done When floors Rust at least 112 and Java at least 108 are above what the repository shows each reader can reach (Rust 109, Java 107), so this epic can never be closed; reword to Rust at least 105, Java at least 95."}
- {file: .engineering/planning/story/reader-rust-kit.md, line: 28, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "at least 112 and at most 8 unclaimed cannot be met: the crate doc names 5 unclaimable features, the epic adds 3 externs, and Rust has no variadic or embedding form for decl.variadic, expr.spread, decl.embedded, so the ceiling is 109."}
- {file: .engineering/planning/story/reader-java-kit.md, line: 23, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "at least 108 and at most 12 cannot be met: the README names 9 features with no Java source form, and stmt.foreign plus 3 externs make 13, a ceiling of 107, or 100 with the 7 refusals by design kept."}
- {file: .engineering/planning/story/reader-check-evaluator-step.md, line: 19, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "The Outcome says the fallback lets stmt.foreign be judged, but the evaluator stops at every foreign statement (E4) and the stmt-foreign case runs its foreign statement, so the only outcome is refused."}
- {file: .engineering/planning/story/reader-c-kit-plain.md, line: 29, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "Claiming stmt.foreign after the evaluator story merges makes a claimed feature that is refused, which conform.rs counts as fail; drop it, which leaves 18 of 20 needed for 77, with decl-variadic, expr-nil and expr-zero needing a variant case."}
- {file: .engineering/planning/story/reader-c-kit-conventions.md, line: 27, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "The candidates include expr.extern and type.extern, which codespec-write-c refuses and the epic lists as gaps for C; move both to Not candidates."}
- {file: .engineering/planning/story/reader-java-quarkus-refusals.md, line: 6, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "The title (at most two refusals) and the summary (jar exceptions read) contradict the acceptance, which allows at most 10 refusals and keeps the 8 jar throws."}
```
