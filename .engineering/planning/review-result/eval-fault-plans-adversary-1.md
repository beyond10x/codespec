---
format: aep.planning-md/3
id: review-result:eval-fault-plans-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on eval-fault-plans
relations:
- reviews: story:eval-fault-plans
revision: 2
---
**Verdict: NEEDS-CHANGE.** I wrote 8 cases and 7 fail, each for the reason it names. All of them are in this commit's new code. The two worst: a trace calls an error "dropped" when it was kept in session memory and returned by the next call, and an `update` event shows a value the caller never sees. In both, later lines of the same trace contradict the earlier claim. The story's five acceptance items still hold.

```
unit: impl/eval-fault-plans @ 847af4f (worktree rd-evfaults, plus my untracked test files)
verdict: NEEDS-CHANGE
cases: executed 17→25, red 7
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (scratch dir, listed below)
needs-coordinator: whether an error fault's partial updates mean "written before the fault" (code, § 8.1) or "left in memory when the call returns" (commit message)
```

**1. Diff.** `git diff --stat HEAD` is empty, so no tracked file changed. Untracked: `crates/eval/tests/adversary_faults.rs` and `crates/eval/tests/fixtures/adv-faults/advfaults.go`. Both are test files.

**2. Cases**, in `~/.local/state/worktree/trees/b10x/codespec/rd-evfaults/crates/eval/tests/adversary_faults.rs`, each run on its own before the suite:
- `an_error_kept_in_memory_older_than_the_call_is_not_reported_dropped`: FAILED. Output: `reported "dropped at example.com/eval/adv-faults.Box.Keep#0.rhs.0"`. The next call's trace line is `{"event":"propagated",...,"fault":"example.com/eval/adv-faults.Box.Keep#0.rhs.0","nth":1}`.
- `an_error_wrapped_in_an_error_type_and_returned_is_propagated`: FAILED. The summary is `"outcome":"handled at example.com/eval/adv-faults.Wrap#1.cond"`, but the call returns `error(OpError{Op: \"atoi\", Err: error(\"injected fault\")})`.
- `an_error_live_when_the_run_stops_is_not_reported_dropped`: FAILED. Output: `the run stopped before \`if err != nil\`, reported "dropped at example.com/eval/adv-faults.Late#0.rhs.0"`.
- `a_write_the_call_undid_before_returning_is_no_partial_update`: FAILED. The trace has `{"event":"update","node":"example.com/eval/adv-faults.Gate.Try#0",...,"value":"true"}`, but `Busy` is false again when the call returns.
- `a_partial_update_shows_what_memory_holds_when_the_fault_fires`: FAILED. The only update is `"value":"&Item{Name: \"a\", State: \"\"}"`, while the later `Get` returns `Item{Name: \"a\", State: \"added\"}`.
- `a_stub_every_call_of_which_a_fault_answers_is_refused`: FAILED. Output: `the stub answers no call and was accepted`.
- `a_plan_entry_with_a_key_given_twice_is_refused`: FAILED. Output: `read as Ok([Fault { at: "a.F#0", nth: At(1), act: Block }])`.
- `nth_counts_the_sites_reaches_over_the_session`: ok. This is a probe that is meant to pass: nth fires at the 2nd reach across session calls, and nth 3 is reported `not reached`.

**3. Suite run** (targeted only), after the cases existed: `cargo test -p b10x-codespec-eval --locked --no-fail-fast --test faults --test adversary_faults`, then `-p b10x-codespec-cli --test eval`:
- `adversary_faults`: `test result: FAILED. 1 passed; 7 failed; ...`
- `faults`: `test result: ok. 10 passed; 0 failed; ...`
- cli `eval`: `test result: ok. 7 passed; 0 failed; ...`
- Exit status: 101 for eval, 0 for cli.
- The "before" count of 17 is the `faults` (10) and cli `eval` (7) binaries in that same run, with my file excluded.

**4. What reaches each finding**
- **Kept error:** a method that stores an error in its receiver, a common Go pattern. `--each` gives every error-returning call an error fault.
- **Wrapped in an error type:** the `PathError`/`OpError` pattern. It is reported differently from `errorf` with `%w`, because `origin()` only looks at `ErrorVal::Message` and interfaces.
- **Run stops with the error live:** any extern without a handler that runs after the faulted call. That is common under `--each`; `strconv.Atoi` itself has no handler.
- **Stale update value:** create, store, then fill in a value. `record_write` renders the pointer when it is written, not when the fault fires, and writes to the new cell are never recorded.
- **Undone write:** a busy flag set and cleared around a call that gets an error fault.
- **Stub shadowed by a `*` fault:** a CLI user who gives both a `--stub` and a `--fault` for one site.
- **Duplicate key in a plan:** only a hand-written plan file reaches it.

**5. Attacked, did not break**
- Counting nth over the run, and an nth beyond reach being `not reached`.
- Two faults at one site (refused); a fault and a stub at one site (the fault answers first, as documented).
- A `block` fault stops the run and later calls do not run. The summary comes before `stop`, which stays the last line.
- Dropping through `_` in `:=`, assignment and a discarded statement result. `propagated` through `return f()`. `wrapped` by `errorf`, which keeps the error's origin.
- Plan parsing: unknown fields, schema version, `nth` 0 / `1.0` / `"1"`, `value` with the wrong arity or type.
- Determinism: the `HashMap` of armed faults is only walked in plan order.
- `--each`: an empty sites file, duplicate sites, and path traversal (a site must be a real node id and `/` is mapped to `-`). The `--each` file collision check exists but is case-sensitive.
- `orders.eval-trace.jsonl` is not in the diff.
- `err.Error()` on a plain error variable stops `unsupported` ("the module has no function .Error"). That is a separate limitation that existed before this unit, so I dropped that case.
- Not tested: whether a value a panic leaves partly built stays in its own cell. I found nothing that changed that path.

**6. Paths written outside the worktree**, all under `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evfaults/adv/`: `adv1.log`, `adv2.log`, `suite.sh`, `suite-eval.log`, `suite-cli.log`. The test binaries' Go reader build went into the tree's `target/tmp/adv-faults-reader`, which is inside the worktree.

**7. Tests I wrote and their result lines (current run)**
- `test a_plan_entry_with_a_key_given_twice_is_refused ... FAILED`
- `test an_error_live_when_the_run_stops_is_not_reported_dropped ... FAILED`
- `test an_error_wrapped_in_an_error_type_and_returned_is_propagated ... FAILED`
- `test a_write_the_call_undid_before_returning_is_no_partial_update ... FAILED`
- `test an_error_kept_in_memory_older_than_the_call_is_not_reported_dropped ... FAILED`
- `test a_partial_update_shows_what_memory_holds_when_the_fault_fires ... FAILED`
- `test a_stub_every_call_of_which_a_fault_answers_is_refused ... FAILED`
- `test nth_counts_the_sites_reaches_over_the_session ... ok`

```findings
- file: "crates/eval/src/fault.rs"
  line: 835
  category: "correctness"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "An error fault whose error is stored into memory older than the call (b.Last = err) gets summary 'dropped at <site>' although it is kept and returned by the next call; the same trace then shows a propagated event for that fault. Evidence: cargo test --test adversary_faults an_error_kept_in_memory_older_than_the_call_is_not_reported_dropped -> reported dropped at example.com/eval/adv-faults.Box.Keep#0.rhs.0. Fix: a write of an error carrying an origin into older memory (record_write) should give an outcome such as 'kept at <node>' instead of the dropped default."
- file: "crates/eval/src/fault.rs"
  line: 797
  category: "correctness"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "record_write renders the written value when the write happens, so a pointer stored into older memory and filled in afterwards (it := &Item{}; s.items[k] = it; it.State = added; send) is reported as update value State empty, while the caller sees State added; the fill-in write is to a cell created in the call and is not recorded at all. Evidence: a_partial_update_shows_what_memory_holds_when_the_fault_fires -> only update value &Item{Name: a, State: empty}, later Get returns State added. Fix: keep the place and render it when the fault fires."
- file: "crates/eval/src/fault.rs"
  line: 567
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "origin() sees only ErrorVal::Message and interfaces, so a fault's error wrapped in an error type (return OpError{Err: err}) loses its origin: the summary says 'handled at Wrap#1.cond' instead of 'propagated to Wrap', while the same wrap done with errorf gives 'propagated to'. Evidence: an_error_wrapped_in_an_error_type_and_returned_is_propagated -> outcome handled at example.com/eval/adv-faults.Wrap#1.cond, results error(OpError{Op: atoi, Err: error(injected fault)})."
- file: "crates/eval/src/session.rs"
  line: 512
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "When the run stops (no handler, step budget, blocked) after an error fault fired and before the error was read, end_call(None) falls through to the default 'dropped at <site>', claiming the module dropped an error it never had the chance to test. Evidence: an_error_live_when_the_run_stops_is_not_reported_dropped -> reported dropped at example.com/eval/adv-faults.Late#0.rhs.0 followed by stop kind no handler. Fix: give a stop its own outcome, for example 'live at stop'."
- file: "crates/eval/src/fault.rs"
  line: 686
  category: "correctness"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "For an error fault the call continues, but the updates are the writes made before the fault, including ones the call undoes before returning; the commit claims updates are writes a failed call left in memory. Evidence: a_write_the_call_undid_before_returning_is_no_partial_update -> update Gate.Try#0 value true, summary updates 1, although Busy is false when Try returns. Needs a decision: snapshot at the fault (doc 8.1) or compare at return (commit message)."
- file: "crates/eval/src/host.rs"
  line: 181
  category: "boundary"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "A stub at a site that a nth * fault answers on every call answers no call, yet is accepted, unlike the existing refusal of a key stub fully shadowed by node stubs. Evidence: a_stub_every_call_of_which_a_fault_answers_is_refused -> the stub answers no call and was accepted."
- file: "crates/eval/src/fault.rs"
  line: 240
  category: "boundary"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "read_faults uses serde_json::from_slice into a Value, so a plan entry with a key given twice is silently read as the last one, for example do closed then do block reads as block. Evidence: a_plan_entry_with_a_key_given_twice_is_refused -> read as Ok([Fault { at: a.F#0, nth: At(1), act: Block }])."
- file: "crates/eval/src/fault.rs"
  line: 366
  category: "judgement"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "The --each check that two sites do not share a trace file compares names exactly, so sites differing only in case (methods Pay and pay, both legal Go) pass the check and on a case-insensitive file system (macOS default) one trace silently overwrites the other in --out. Code path: trace_file plus files.contains; no test written, Linux only here."
```
