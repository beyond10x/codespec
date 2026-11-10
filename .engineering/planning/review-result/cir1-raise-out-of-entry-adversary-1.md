---
format: aep.planning-md/3
id: review-result:cir1-raise-out-of-entry-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on cir1-raise-out-of-entry
relations:
- reviews: story:cir1-raise-out-of-entry
revision: 2
---
unit: story:cir1-raise-out-of-entry, tree `rd-java-quarkus` at `21ba342` plus 3 untracked adversary test files
verdict: NEEDS-CHANGE
cases: executed 43→49, red 5
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path, the assigned scratch directory
needs-coordinator: `AdversaryRaiseTest.java` is not wired into `task adapters-check`, so it runs only by hand (command in part 2). I took no worktree lease.

**1. Diff.** `git --no-pager diff --stat` is empty: no tracked file was changed. `git status --short` shows only new test files:
```
?? adapters/java/test/codespec/read/AdversaryRaiseTest.java
?? crates/model/tests/adversary_raise.rs
?? crates/query/tests/adversary_raise.rs
```

**2. Cases added.** Each was run alone first, and all are red now.
- `crates/model/tests/adversary_raise.rs`
  - `adversary_a_raising_function_used_as_a_value_in_a_variable_initializer_is_refused` is red: `admitted: the variable initializer uses the raising check as a value and Run, exported, no entry, no raises, calls it`
  - `adversary_a_raising_function_called_by_a_variable_initializer_is_refused` is red: `admitted: a variable initializer calls depth, which raises`
  - `adversary_a_raise_reached_from_init_is_refused` is red: `admitted: init, which no entry names and the loader runs, raises`
  - `adversary_the_base_document_reads` is a green control. It shows the refusals above come from what each test adds.
- `crates/query/tests/adversary_raise.rs`
  - `adversary_check_and_the_lock_property_agree_on_a_lock_held_across_a_call_that_raises` is red: `check lock-without-unlock reports Balanced (["example.com/locks.Balanced", "example.com/locks.Early", "example.com/locks.Loop"]); the lock property does not: example.com/locks.Early ... Lock at example.com/locks.Early#0.x has a path to return without Unlock`
- `adapters/java/test/codespec/read/AdversaryRaiseTest.java`
  - Run by hand: `javac` of the src tree plus the file, then `java -cp <classes> codespec.read.AdversaryRaiseTest <work>` with `CODESPEC_JAVA_CLASSPATH` set.
  - Red at HEAD (exit 1): `FAIL adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before: [ { "pos": "Res.java:17", "what": "long a = find(k);", "reason": "call of Res.find, which raises: ..." } ]`
  - Green against the base reader (merge-base `8f5d084`, compiled into scratch): `ok adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before`, exit 0.

**3. Suite run** (targeted, after the cases existed, with `--no-fail-fast`):
- `cargo test -p b10x-codespec-model --test raise --test adversary_raise`
  - `adversary_raise`: 1 passed, 3 failed
  - `raise`: 10 passed
- `cargo test -p b10x-codespec-query --test paths --test query --test adversary_raise`
  - `adversary_raise`: 1 failed
  - `paths`: 13 passed
  - `query`: 20 passed
- Exit 101.

The "before" count, 43, is these runs with my files deselected (10 + 13 + 20). The "after" count, 49, adds my 5 Rust cases and the Java case. The 979 figure is the implementor's full workspace run, which I did not repeat.

**4. Findings**
- **Variable initializers are not checked** (`crates/model/src/lib.rs:2584`). `check_raises` looks only at `inits`, a call list the strict reader never compares with the `vars[].init` expressions. So an initializer can use or call a raising function, and an exported non-entry function can then carry the raise out with no `raises`. The function's own doc says "nor called by ... a variable's initializer".
  - What reaches it: any document passed to `codespec validate` or `read`. No shipped reader writes this today; the Java reader refuses method references in a project.
  - Fix: walk `vars[].init` with `expr_callees`, and treat every hit as an escape.
- **`init` is admitted as if an entry reached it** (`crates/model/src/lib.rs:2643`). An unexported `init` that reaches a raise is accepted, but the loader runs `init` (the evaluator's `run_inits`), not a framework. This contradicts the acceptance line "admits raise only in a function ... only entries reach".
  - What reaches it: a hand-written or future-reader document. Nothing writes it today.
  - Fix: refuse a non-empty `raises` on `init`.
- **The Java reader refuses calls that do not raise** (`adapters/java/src/codespec/read/Reader.java:1346`). `mayRaise` counts any `throw` in an entry, including a project checked exception that is read as an error result. A call of such an entry is now refused as "which raises", although the callee lists no `raises`. The base reader read the same call with no refusal, so this is a regression.
  - What reaches it: any Quarkus resource method that throws a project exception and is called by another project method.
  - Fix: count only a throw that `throwStmt` maps to `raise` (a jar's unchecked exception, not a project class).
- **cq and `check` disagree** (`crates/query/src/facts.rs:1210`). The cq flow treats a `raise` statement as an exit, but not a call of a function that lists `raises`. `codespec check lock-without-unlock` counts that call as a way out (`checks.rs`, "may leave through ..., which raises"). So the shipped lock property misses `Balanced` and `Loop`, which `check` reports.
  - What reaches it: `codespec query` with `properties/lock.cq` on any module with a raising callee.

**5. Attacked, not broken**
- Writers: a caller with no `raise` of its own that calls a raising function. Rust lists it in `refusals.json` ("calls check, which is refused"), C exits 1 and lists it, Go refuses the whole module.
- Strict reader: a raise in a lambda or nested lambda; use as a value; opaque callers; `raises` sorting, duplicates and non-extern entries.
- Java reader: `try` and rethrow are refused; a throw in a lambda or a non-entry helper is refused. A method reference to a raising entry is refused as a reference, so no invalid document comes out.
- Evaluator: a raise stops the run, and a session stops on it.

**6. Paths written outside the worktree**
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/raise/adv/`: probe projects `proj`, `proj2`; `base-src`, `base-classes`; `jt-classes`, `jt-base-classes`, `jt-work`, `jt-base-work`; `caller.cir.json`, `caller.passed.json`, `rust-out`, `c-out`; `*.cir.json` and `*.err` logs.
- The Go writer's `go-out` would be in the same directory, but it refused and wrote nothing.

**7. Findings block.** One line per finding (test file, test name, red line):
- `crates/model/tests/adversary_raise.rs`, `adversary_a_raising_function_used_as_a_value_in_a_variable_initializer_is_refused`: `admitted: the variable initializer uses the raising check as a value and Run, exported, no entry, no raises, calls it`
- `crates/model/tests/adversary_raise.rs`, `adversary_a_raise_reached_from_init_is_refused`: `admitted: init, which no entry names and the loader runs, raises`
- `adapters/java/test/codespec/read/AdversaryRaiseTest.java`, `adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before`: `reason: call of Res.find, which raises`
- `crates/query/tests/adversary_raise.rs`, `adversary_check_and_the_lock_property_agree_on_a_lock_held_across_a_call_that_raises`: `check lock-without-unlock reports Balanced (...); the lock property does not`

```findings
- file: "crates/model/src/lib.rs"
  line: 2584
  category: "correctness"
  severity: "warning"
  origin: "introduced"
  verdict: "NEEDS-CHANGE"
  message: "check_raises reads only the inits call list, never vars[].init expressions, so a variable initializer can call or take as a value a raising function and an exported non-entry caller carries the raise out (adversary_a_raising_function_used_as_a_value_in_a_variable_initializer_is_refused, adversary_a_raising_function_called_by_a_variable_initializer_is_refused)"
- file: "crates/model/src/lib.rs"
  line: 2643
  category: "correctness"
  severity: "warning"
  origin: "introduced"
  verdict: "NEEDS-CHANGE"
  message: "an unexported init that reaches a raise is admitted though the loader, not an entry, runs it (adversary_a_raise_reached_from_init_is_refused)"
- file: "adapters/java/src/codespec/read/Reader.java"
  line: 1346
  category: "correctness"
  severity: "warning"
  origin: "introduced"
  verdict: "NEEDS-CHANGE"
  message: "mayRaise counts any throw in an entry, so a call of an entry that only throws a project checked exception is now refused as one which raises, where the base reader read it (AdversaryRaiseTest adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before)"
- file: "crates/query/src/facts.rs"
  line: 1210
  category: "correctness"
  severity: "warning"
  origin: "introduced"
  verdict: "CONFIRMED"
  message: "the cq flow treats a raise statement as an exit but not a call of a function listing raises, so the lock property misses a lock held across such a call that check lock-without-unlock reports (adversary_check_and_the_lock_property_agree_on_a_lock_held_across_a_call_that_raises)"
```
