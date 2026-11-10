---
format: aep.planning-md/3
id: review-result:qe-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2 (query and evaluator)
relations:
- reviews: story:query-core
- reviews: story:query-paths-properties
- reviews: story:cir1-defer-receiver-root
- reviews: story:eval-core
- reviews: story:eval-conformance-runner
- reviews: story:eval-handlers-traces
- reviews: story:eval-fault-plans
revision: 1
---
needs-revision
story:eval-core — bullet 1 requires every kit case to print its `expected.txt`, including `stmt-foreign` (`defer done()`, expects `ok 3`) and the extern cases such as `expr-extern` (expects `ok "ababab3"`), while bullet 2 stops the run at any foreign statement or unhandled extern and the Out of Scope section excludes handlers, so the step can pass and fail on the same run — .engineering/planning/story/eval-core.md:20
story:query-core — "A query whose answer rests on a function with no body says so in its output" names no fixture and no output text to look for, so it reads the same before the work as after it — .engineering/planning/story/query-core.md:45
story:query-paths-properties — the § 6 examples "parse and run on a checked-in fixture, byte-equal across two runs" names no expected result, so an empty or wrong answer printed twice passes — .engineering/planning/story/query-paths-properties.md:26
story:query-paths-properties — the Outcome promises `path(...)` "over control flow or the call graph" and properties "over the calls of a body or a call path", but the bullets do not say which graph the path query uses and test a property over a body only — .engineering/planning/story/query-paths-properties.md:22
story:cir1-defer-receiver-root — "and leaves the list of checks waiting for a design section" reads either as removed from the list or as the list unchanged, and `story:query-core` asserts `lock-without-unlock` is in that list (.engineering/planning/story/query-core.md:39), so the two outcomes check opposite things — .engineering/planning/story/cir1-defer-receiver-root.md:29
story:cir1-defer-receiver-root — the bullet says `recv_var` is written "on method and interface call refs" but its test asserts the field only on `c.mu.Lock()` and `defer c.mu.Unlock()`, so the interface half of the claim has no check — .engineering/planning/story/cir1-defer-receiver-root.md:27

What I read: 7 of 7 ids (`aep plan artifact show` on each), plus `review-result:qe-acceptance-round-1`, `story:builtin-defect-checks`, `aep plan artifact kinds`, `docs/query-language.md` §§ 1, 6, 8, 9, `docs/evaluator.md` §§ 2, 3, 6.1 to 6.3, 8.1, 11, the kit cases `stmt-foreign` and `expr-extern`, `website/data/conformance` (8 reports) and `crates/check/tests/fixtures`.

What I could not establish:
- Round-1 fixes that now hold: the "eight existing reports" count matches `website/data/conformance`; the `--each`, dropped-error fixture, `recv_var` and `--explain` outputs are now named.
- Whether the evaluator's built-in pure handlers (`docs/evaluator.md` § 6.1) are meant to be inside `story:eval-core`. The finding on eval-core stands either way, because its Out of Scope excludes handlers and the `stmt-foreign` case is a foreign statement, not an extern.
- Out of my lane, not counted toward the verdict: `--explain` in `story:query-core` prints strata order, while `docs/query-language.md` § 7 says it prints a derivation tree per finding (design critic).
- Out of my lane, not counted toward the verdict: the dependency edges and the shared `Taskfile.yml`, `lib.rs` and `Cargo.toml` surfaces (scope and parallel-safety critics).

```findings
- file: .engineering/planning/story/eval-core.md
  line: 20
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "bullet 1 requires every kit case to print its expected.txt, including stmt-foreign (defer done(), expects ok 3) and the extern cases such as expr-extern (expects ok \"ababab3\"), while bullet 2 stops the run at any foreign statement or unhandled extern and the Out of Scope section excludes handlers, so the step can pass and fail on the same run"
- file: .engineering/planning/story/query-core.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"A query whose answer rests on a function with no body says so in its output\" names no fixture and no output text to look for, so it reads the same before the work as after it"
- file: .engineering/planning/story/query-paths-properties.md
  line: 26
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the § 6 examples \"parse and run on a checked-in fixture, byte-equal across two runs\" names no expected result, so an empty or wrong answer printed twice passes"
- file: .engineering/planning/story/query-paths-properties.md
  line: 22
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises path(...) over control flow or the call graph and properties over the calls of a body or a call path, but the bullets do not say which graph the path query uses and test a property over a body only"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"and leaves the list of checks waiting for a design section\" reads either as removed from the list or as the list unchanged, and story:query-core asserts lock-without-unlock is in that list, so the two outcomes check opposite things"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet says recv_var is written on method and interface call refs but its test asserts the field only on c.mu.Lock() and defer c.mu.Unlock(), so the interface half of the claim has no check"
```
