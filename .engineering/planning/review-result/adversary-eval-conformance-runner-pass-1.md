---
format: aep.planning-md/3
id: review-result:adversary-eval-conformance-runner-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the evaluator's conformance runner and differential step
relations:
- reviews: story:eval-conformance-runner
revision: 1
---
unit: story:eval-conformance-runner, adversary pass 1, unit commit d32521c (base 767110c) plus the untracked `crates/eval/tests/adversary3.rs`
verdict: NEEDS-CHANGE
cases: executed 52→56, red 3
origin: introduced 3 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 (scratch directory, listed in part 6)
needs-coordinator: no

## 1. Diff stat

`git status --short` shows `?? crates/eval/tests/adversary3.rs` and nothing else. `git diff --stat` on tracked files is empty. Against `/dev/null`: `crates/eval/tests/adversary3.rs | 199 +++`. It is the only file changed, and it is a test file. I ran rustfmt on that file only; `cargo fmt --all --check` and `cargo clippy -p b10x-codespec-eval --all-targets -D warnings` both exit 0.

## 2. Cases added (`crates/eval/tests/adversary3.rs`)

Each case was first run alone with `cargo test -p b10x-codespec-eval --test adversary3`. All four are in the same state now as in that run.

**1. `adv3_a_probes_setup_calls_mostly_succeed_as_the_diff_doc_says`: red.**
- What it checks: `diff.rs:55` says setup calls "mostly succeed". The case takes each probed method that cannot fail itself, where any `err` line must come from a failed setup call, and requires fewer than half of its seeds to end that way.
- Output:
  `left: ["inventory Inventory.Find: 132 of 200 seeds end in a setup call's error", "inventory Inventory.StockOf: 122 of 200 …", "inventory Inventory.ValueOf: 131 of 200 …", "inventory Inventory.LowStock: 128 of 200 …", "inventory Inventory.Reserved: 144 of 200 …"]`
- Further measured (output counts below, from a separate script run in scratch, not from this case): of 200 seeds, `StockOf` returns a nonzero stock once, `ValueOf` returns nonzero 3 times, `Find` finds an item 7 times, and `Receive` succeeds 5 times.

**2. `adv3_every_entry_prints_more_than_one_distinct_line_over_its_seeds`: red.**
- Output: `left: ["series Series.Reverse: all 200 seeds print \"ok\""]`
- A method with no results is probed as a bare call (`diff.rs:655`), and the receiver is never printed. So the 200 Reverse seeds of `eval-diff` compare nothing. Reverse is only seen indirectly, as a setup slot in other probes.

**3. `adv3_a_refusal_listed_for_an_example_not_run_fails`: red.**
- Setup: the evaluator's own writer and runner stand in for Go and Rust.
- Two controls pass: with no listing the run exits 0, and the stale listing `series:rust:Steps` exits 1.
- `--refused inventory:rust:Inventory.Find` on a run of `series` alone: `assertion left != right failed: a listing for an example not run is accepted: eval-diff: series: … byte-equal / left: Some(0)`.
- Cause: `codespec-eval-diff.rs:224` checks only listings whose example is on the command line.

**4. `adv3_mut_strings_order_byte_by_byte_not_by_length`: green on the tree, red on a mutant.**
- What it checks: the kit's `op-lt` case with `string` operands. Expected output is `"ok false\nok true\nok true\n"`, matching Go's byte order.
- The mutant: `interp.rs:2165` changed to compare length first. It was applied to a copy of `crates/eval` in scratch, never to the tree.
- Red output on the mutant: `left: "ok true\nok false\nok true\n" / right: "ok false\nok true\nok true\n"`.

## 3. Suite run, after all four cases existed

- Command: `cargo test --locked --no-fail-fast -p b10x-codespec-eval`, exit 101 (`error: 1 target failed`).
- Per binary: lib 6, adversary1 14 (1 ignored), adversary2 14 (1 ignored), adversary3 `FAILED. 1 passed; 3 failed`, crossings 1, diff 3, ids 2, kit 2, refusals 2, runner 5, stops 3.
- So 56 cases ran. The "before" figure of 52 is the same per-binary counts with adversary3 excluded; the earlier three-case run gave the same counts.

**Mutation probe.** For each mutant I built a mutated copy of `crates/eval` in scratch and ran it on every kit case and on the `eval-diff` probe calls. Its output was compared with the unmutated evaluator's, which `eval-diff` had found byte-equal to Go and Rust.

| Mutant | Kit | eval-diff |
|---|---|---|
| unmutated control | 0 failing | 0 differing |
| `%` as `rem_euclid` | sem-rem-sign fails | **0 differing** |
| string `+` with operands swapped | 6 cases fail | **0 differing** |
| map set does not overwrite an existing key | **0 failing** | 46 inventory lines differ (only eval-diff catches it) |
| strings ordered by length first | **0 failing** | **0 differing (survives both)** |
| div-floor, `<` as `<=`, `>=` as `>`, mul/add saturating, platform int as 32 bits | caught | caught |

## 4. Findings (covering d32521c plus the untracked test file)

| # | file:line | Severity | Verdict / origin | Finding | What reaches it |
|---|---|---|---|---|---|
| F1 | `crates/eval/src/diff.rs:55` | warning | NEEDS-CHANGE / introduced | The doc says setup calls "mostly succeed". On inventory, 61–72% of seeds end in a setup error, so 5 of the 11 inventory methods are compared on fewer than 80 of their 200 seeds, mostly against an empty inventory. Either the setup draws are fixed (for example, a failed setup call skipped instead of ending the probe, or `AddItem` drawn before `Receive`/`Reserve`) or the doc and the claimed 200 seeds are corrected. | `task eval-diff` in `task check` |
| F2 | `crates/eval/src/diff.rs:655` | warning | CONFIRMED / introduced | A probe of a method with no results prints no state, so `Series.Reverse`'s own 200 seeds are constant `ok`. The suggested fix is for the probe to return the receiver's printable fields after the call. | `task eval-diff` |
| F3 | `crates/eval/src/bin/codespec-eval-diff.rs:224` | note | CONFIRMED / introduced | A `--refused` listing for an example not on the command line passes silently. | Only when an example is dropped from the Taskfile line while its listing stays |
| F4 | `crates/eval/src/interp.rs:2165` | note | CONFIRMED / pre-existing | The kit tests `<` on integers only, and the probe sort only ever orders `"a"` and `"b"`. A wrong string order therefore passes both the kit and `eval-diff`, while `op.lt` and `op.gt` are claimed. Case 4 catches it. The kit's `op-lt` and `interp.rs` are unchanged since the base. | Any program comparing strings of different lengths |
| F5 | `adapters/rust/src/write.rs:3808` | note | CONFIRMED / pre-existing | `Inventory.Find` is compared only between cir and Go, never Rust, so the acceptance's three-way comparison is not met for that one function. The unit lists this openly. The refusal is at the same line at base. | `task eval-diff --refused inventory:rust:Inventory.Find` |

## 5. Attacked and not broken

- **Determinism:** two `eval-diff` runs gave byte-identical `calls.json` and `probes.cir.json`, equal to the implementor's, with exit 0 both times. Seeds use FNV of the probe name XOR the seed, integer arithmetic only, no time.
- **`codespec conform --check website/data/conformance/writer-cir.json`:** reports "current", 125 claimed, 125 pass, 1 refused unclaimed, 1 pass unclaimed. The pass-unclaimed one is `intrinsic.append`, which matches the report and the reason given.
- **Sorting map-ordered results:** the sort is applied only to `LowStock`, whose body holds a position listed as unordered iteration. It hides only order, and `Series.Sorted` is not re-sorted.
- **A stop or refusal in `eval-diff`:** the cir writer's refusal and `codespec-run-cir` exiting 1 both fail the run; neither is reported as a pass.
- **Release workflow:** replaying the smoke step for `cir` locally passes (`write`, run, equal to `expected.txt`). The archive adds only `codespec-write-cir` and `codespec-run-cir`; `codespec-eval-diff` is built but not shipped.
- **Constant folding claim:** this limit was already found by the earlier adversary and is attributed to the Go reader (the ignored case `adv_constant_expressions_are_exact_as_go_folds_them`), so I did not raise it again.
- **Environment note (does not affect the unit):** `codespec-run-go` fails to build under `~/.cache/scratch` because of a stray `~/.cache/.git`. That is why my `eval-diff` runs used build directories under the worktree's `target/`.

## 6. Paths written outside the worktree

- `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/qe-eval-runner/adv1` (281 MB): logs, scripts, `ws/` (the mutation workspace and its `target/`), `eval-pristine/` and `kitrun/`.

Inside the worktree's build output (`target/`), not tracked by git:
- `target/adv1-evaldiff1`, `target/adv1-evaldiff2`, `target/adv1-smoke`, `target/adv1-conform-cir`
- `target/tmp/adv3-adapters`, `target/tmp/adv3-eval-diff`
- `target/adapters/codespec-{write,run}-cir`, overwritten with a fresh build of the same source.

I released my session lease on the worktree.

```findings
- file: crates/eval/src/diff.rs
  line: 55
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: setup calls end 61-72% of seeds of five inventory probes in a setup error, contradicting "mostly succeed", so those functions are compared on under 80 of their 200 seeds and StockOf sees a nonzero stock once
- file: crates/eval/src/diff.rs
  line: 655
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a probe of a method without results prints no receiver state, so Series.Reverse's own 200 seeds all print "ok" and compare nothing
- file: crates/eval/src/bin/codespec-eval-diff.rs
  line: 224
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a --refused listing naming an example not on the command line is accepted silently instead of failing as stale
- file: crates/eval/src/interp.rs
  line: 2165
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: a mutant ordering strings by length first survives the kit and eval-diff because op.lt is tested on integers only and probes sort only "a" and "b"
- file: adapters/rust/src/write.rs
  line: 3808
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: Inventory.Find is compared only between cir and Go because the Rust writer refuses Option results, so the three-way acceptance is not met for that function
```
