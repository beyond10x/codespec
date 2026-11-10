---
format: aep.planning-md/3
id: review-result:eval-handlers-traces-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on eval-handlers-traces
relations:
- reviews: story:eval-handlers-traces
revision: 2
---
unit: commit 567233f (handlers, stubs, sessions, traces), checked in the working tree of impl/eval-handlers-traces, which contains only my two untracked test paths
verdict: NEEDS-CHANGE
cases: executed 10→15, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: none

Short verdict: I broke it 5 ways, all in sessions and stubs. The worst one: a session gives the wrong answer and does not stop. A pointer-receiver method called on a struct value bound in the session changes a copy, not the bound value.

1. `git --no-pager diff --stat` prints nothing, because tracked files are untouched. `git status --short` shows only test paths:
```
?? crates/eval/tests/adversary_handlers.rs
?? crates/eval/tests/fixtures/adv-session/
```

2. **Cases added** (I wrote them first, then ran the file alone; all 5 are red now). The file is `crates/eval/tests/adversary_handlers.rs` and the fixture is `crates/eval/tests/fixtures/adv-session/session.go`.
   - `adv_a_pointer_method_on_a_bound_value_changes_that_value`: NewCounter, then Counter.Inc and Counter.Get on `c`, must return 1. It returns `{"event":"return","n":3,"results":["0"]}`.
   - `adv_a_skipped_call_that_rebinds_a_name_does_not_leave_its_old_value`: x is bound to a string, a call that would rebind x is skipped, then `Twice($x)` must be skipped too. It runs instead with `{"event":"call","n":4,"func":"Twice","args":["\"ann\""]}` and then stops `unsupported: operator * on these values`.
   - `adv_a_bound_int_for_an_error_parameter_is_refused`: `Msg($n)` with n an int and Msg taking an `error` must be refused. It is accepted and then stops `unsupported ... the module has no function .Error`.
   - `adv_writing_an_error_into_the_trace_does_not_run_its_error_method`: after `Fail()`, `Rendered()` must return 0. It returns `{"event":"return","n":2,"results":["1"]}`.
   - `adv_two_stubs_for_one_extern_are_refused`: two `--stub` values for `os.Getenv` must be refused. They are accepted, the run prints `ok "b/b"` and the first stub is dropped without a word.

   Red output from that first run: `test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.81s`.

3. **Suite run** (targeted only):
   - `cargo test -p b10x-codespec-eval --locked --no-fail-fast --test sessions --test stops` without my file: sessions 4 passed, stops 6 passed, exit 0. That is the "before" count of 10.
   - The same command plus `--test adversary_handlers`: adversary_handlers 0 passed, 5 failed; sessions 4 ok; stops 6 ok; exit 101.
   - `cargo test -p b10x-codespec-cli --locked --no-fail-fast --test eval`: 4 passed, exit 0.

4. **Findings** (all on 567233f; how each is reached):
   - `crates/eval/src/session.rs:433`: `receiver()` copies a bound value into a new cell for a pointer receiver, so `c.Inc()` never changes `c`. Any session whose constructor returns a struct by value reaches it. It is a wrong answer, not a stop, which goes against the "shared state" claim. The orders example avoids it only because `New` returns `*Service`. Fix: keep bound values in heap cells and pass that cell's address.
   - `crates/eval/src/session.rs:331`: a skipped call does not remove its `bind`, but a panicking call does (line 386). A later `$x` therefore gets the older value, possibly of a type `plan` never checked. Any session that rebinds a name after a panic reaches it.
   - `crates/eval/src/session.rs:236`: if the parameter is an interface, `plan` accepts any bound value, whether or not its type implements the interface. The module doc promises this is refused. Any session that passes a bound value to an `error` or interface parameter reaches it.
   - `crates/eval/src/interp.rs:307`: writing an error into the trace runs the module's `Error` method. That changes module state, uses up steps from the budget, and adds to `Run.stubbed` counts when `Error` calls a stubbed extern. It is reached by any session result, argument, sent value or extern argument that holds a `to_error` value.
   - `crates/eval/src/host.rs:140`: two stubs for the same site are both accepted and the last one wins; the dropped stub is not reported anywhere. A key stub hidden at every site by node-id stubs is not reported either. Reached by repeating `--stub`.

5. **Attacked and could not break:**
   - `--env` parsing: `A=b=c` gives `b=c`; `=x` and `A` are refused; an empty value makes LookupEnv return `("", true)`.
   - TrimSpace, ToUpper, ToLower match Go on ASCII, including `\v`, `\f`, NUL and the empty string; any non-ASCII byte (0x85, invalid UTF-8) stops as `unsupported`.
   - Stubs: wrong arity or type is refused, as are a non-extern, a pure module function, a node id from another module, and a value out of range for an integer width.
   - Trace: field order is fixed, the HashMaps are only used for lookups, and node ids are structural paths, so the output is deterministic. Every string goes through serde, so lines stay valid JSON. `defer` and `go` are outside cir/1, so held locks, unrun defers and goroutine scheduling cannot arise.
   - `--calls` path: unchanged apart from the new handlers (by reading the code against d0093ce).
   - Stubbed lines go to stderr only; stdout stays the run's output.

6. **Paths written outside the worktree** (all under `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evtraces/adv/`): `case.log`, `suite.sh`, `before.log`, `after.log`, `cli.log`, `suite.done`. The build also wrote `target/tmp/adv-handlers-reader/` inside the tree's `target/`. I took no worktree lease of my own; the coordinator's tree was used as handed over.

7. Findings block:

```findings
- file: "crates/eval/src/session.rs"
  line: 433
  category: "acceptance"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "A pointer-receiver method on a bound struct value runs on a fresh copy, so session state is not shared and the run answers wrongly without stopping; adv_a_pointer_method_on_a_bound_value_changes_that_value gets results [0] for NewCounter; c.Inc(); c.Get(), Go gives 1"
- file: "crates/eval/src/session.rs"
  line: 331
  category: "boundary"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "A skipped call keeps the earlier binding of its bind name (a panicking one removes it), so a later $x gets a stale value of a type plan never checked; adv_a_skipped_call_that_rebinds_a_name_does_not_leave_its_old_value shows call n4 Twice args [ann] then stop unsupported operator *"
- file: "crates/eval/src/session.rs"
  line: 236
  category: "contract-drift"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "plan accepts any bound value for an interface parameter, though the module doc promises refusal of a bound value of another type; adv_a_bound_int_for_an_error_parameter_is_refused: Msg($n) with n int runs and stops unsupported the module has no function .Error instead of being refused"
- file: "crates/eval/src/interp.rs"
  line: 307
  category: "judgement"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "Writing an error value into the trace runs the module's Error method, which mutates module state, uses step budget and can inflate Run.stubbed; adv_writing_an_error_into_the_trace_does_not_run_its_error_method: Rendered() returns 1 after Fail(), Go gives 0"
- file: "crates/eval/src/host.rs"
  line: 140
  category: "boundary"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "Two stubs for one site are both accepted and the last silently replaces the first, which is never reported; adv_two_stubs_for_one_extern_are_refused: os.Getenv=a and os.Getenv=b accepted, output ok b/b"
```

Tests written, each with its own result line from the first run:
- `adv_a_pointer_method_on_a_bound_value_changes_that_value ... FAILED`
- `adv_writing_an_error_into_the_trace_does_not_run_its_error_method ... FAILED`
- `adv_a_skipped_call_that_rebinds_a_name_does_not_leave_its_old_value ... FAILED`
- `adv_a_bound_int_for_an_error_parameter_is_refused ... FAILED`
- `adv_two_stubs_for_one_extern_are_refused ... FAILED`
