---
format: aep.planning-md/3
id: review-result:query-paths-properties-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on query-paths-properties
relations:
- reviews: story:query-paths-properties
revision: 2
---
I broke the unit with three red cases. Each one gives a wrong answer, not a refusal: two state a result as definite when the code it depends on was never read, and one silently drops a call-graph entry. Commit 4b33c7a on `impl/query-paths-properties`.

```
unit: query paths and properties, commit 4b33c7a; findings cover that commit plus the uncommitted crates/query/tests/adversary_paths.rs
verdict: NEEDS-CHANGE
cases: executed 30→33, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: none
```

1. **Diff.** `git diff --stat` is empty and `git status` shows only `?? crates/query/tests/adversary_paths.rs`, a test file. I changed no implementation file. The cases edit the existing fixtures in memory, so nothing went under `fixtures/adv-paths/`.

2. **Cases, run alone first** (`cargo test -p b10x-codespec-query --test adversary_paths`). All three are red now; this is that run's output.
   - `a_negated_rule_over_a_positive_path_stays_possible_across_a_call_nothing_links`: `!reach_write(EntryA)`, where `reach_write` uses `path(F -> G, over: calls)`, must be "possible" when EntryA calls a function value.
     ```
     via path:    "example.com/calls.EntryA\n"
     via reaches: "example.com/calls.EntryA\tpossible: example.com/calls.EntryA#0.x at example.com/calls/calls.go:4 (call of a function value, not linked by calls)\n"
     ```
     The inline `!path(...)` and the same rule written with `reaches` both say "possible"; only the path version comes back definite.
   - `a_deferred_unlock_with_no_receiver_does_not_give_a_definite_error`: a Lock released only by `defer mu.Unlock()` (a foreign statement listing its calls, with no receiver) must not give a definite error.
     ```
     a definite error for a Lock released by defer, whose receiver nothing read: example.com/locks.Balanced	example.com/locks.Balanced#1	Lock at example.com/locks.Balanced#0.x has a path to return without Unlock
     ```
     A precondition in the case confirms the deferred Unlock is there as the listed call `#1.calls.0`.
   - `a_calls_property_still_checks_an_entry_that_calls_itself`: add a self-call to ServeUsers in the layers fixture and its expected row should stay.
     ```
     a recursive entry is no longer checked: ""
     ```

3. **Suite, run after the cases existed:** `cargo test -p b10x-codespec-query --locked --no-fail-fast --test paths --test query --test adversary_paths` → paths `11 passed`, query `19 passed`, adversary_paths `FAILED. 0 passed; 3 failed`, exit 101. Deselecting my file gives 30, which is the "before" count.

4. **Findings:**
   - `engine.rs:2385`: in the upper bound, a positive path only adds its "possible" row when the end is unbound. With the end bound and no linked path, it adds nothing, so negating it through a rule comes back definite. That breaks the evidence rule (design § 4.3) and the unit's own claim. It is reached by any rule that wraps `path(...)` and is then negated. **Fix:** when the end is bound and the walk reached functions with unknown calls, add a row that rests on those calls, as `reaches` already does.
   - `engine.rs:2431`: the register check compares against a listed call's receiver, which the reader leaves empty (`facts.rs:1760`). The comparison fails and the run reaches the return as a definite error. This is reached by idiomatic Go, `mu.Lock(); defer mu.Unlock()`, with the design's own `lock.cq`. **Fix:** treat a register compared with a receiver nothing read as a possible match that names the site, or refuse the comparison.
   - `engine.rs:2573`: an entry is "a function no linked call reaches", so a self-recursive function, or a mutually recursive pair with no outside caller, is never an entry. Its call paths are never checked and nothing reports that. It is reached by any recursive exported function in a library with no main. **Fix:** count only callers from outside the function's own strongly connected component.

5. **Attacked and could not break:**
   - `max` boundaries and zero-edge paths.
   - The witness is the least of the shortest paths and is deterministic: BFS with sorted successors.
   - Loops, unreachable code, infinite loops and early returns in properties.
   - Overlapping transitions: every matching transition fires, and the output is deterministic.
   - Recursion through an avoid body is refused, because the avoid relation is treated like a negation in stratification.
   - The `=>`/`:=` tokens and a relation named `path` parse correctly.
   - Diamond chains don't blow up: the walks are linear and the number of runs is polynomial.
   - Not measured: `reaches_path` builds every pair with its full witness, whatever the query binds (`engine.rs:1946`).

6. **Paths written outside the worktree:** all five are in `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/qpaths/adv/`:
   - `syntax.diff`
   - `engine.diff`
   - `rest.diff`
   - `run1.log`
   - `run2.log`

   The worktree lease I took was released when I finished.

```findings
- file: "crates/query/src/engine.rs"
  line: 2385
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "When a positive path's end is bound and no linked path reaches it, the upper bound adds nothing, so a rule over path(...) that is then negated reads as definite although a call nothing links could reach the end; this breaks the evidence rule of design section 4.3 and the unit's own claim that answers across unread code are possible. Evidence: cargo test -p b10x-codespec-query --test adversary_paths, a_negated_rule_over_a_positive_path_stays_possible_across_a_call_nothing_links gives via path example.com/calls.EntryA with no possible marker, while the reaches version and the inline !path both print possible: example.com/calls.EntryA#0.x."
- file: "crates/query/src/engine.rs"
  line: 2431
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "A register is compared with the receiver of a call in a foreign statement's call list, which the reader leaves empty (facts.rs:1760), so mu.Lock() followed by defer mu.Unlock() gives a definite lock_released error instead of a possible one naming the site. Evidence: a_deferred_unlock_with_no_receiver_does_not_give_a_definite_error prints example.com/locks.Balanced, example.com/locks.Balanced#1, Lock at example.com/locks.Balanced#0.x has a path to return without Unlock, with no possible marker."
- file: "crates/query/src/engine.rs"
  line: 2573
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "A calls-scope property starts only at functions no linked call reaches, so a self-recursive or mutually recursive top function is never an entry and its call paths are silently left unchecked. Evidence: a_calls_property_still_checks_an_entry_that_calls_itself, after adding a ServeUsers self-call to the layers fixture the expected row disappears: a recursive entry is no longer checked: empty output."
```

Tests written, each one's result line verbatim, all in `~/.local/state/worktree/trees/b10x/codespec/rd-qpaths/crates/query/tests/adversary_paths.rs` and uncommitted:
- `test a_negated_rule_over_a_positive_path_stays_possible_across_a_call_nothing_links ... FAILED`
- `test a_deferred_unlock_with_no_receiver_does_not_give_a_definite_error ... FAILED`
- `test a_calls_property_still_checks_an_entry_that_calls_itself ... FAILED`
