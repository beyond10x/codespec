---
format: aep.planning-md/3
id: review-result:adversary-eval-core-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:eval-core'
relations:
- reviews: story:eval-core
revision: 2
---
unit: story:eval-core, worktree `qe-eval-core` at d0523cc (`impl/eval-core`), plus my untracked test files
verdict: NEEDS-CHANGE
cases: executed 67→82, red 11
origin: introduced 13 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 (scratch directory `adv2`, 521 MB)
needs-coordinator: F14 (generated site files are still in the unit's diff, see pass-1 F16); the worktree `target/` now also holds my mutant-copy build artifacts (stable rustc 1.99.1)

**1. Diff.** `git --no-pager diff --stat` prints nothing, so no tracked file changed. `git status --short` shows only my new files:
```
?? crates/eval/tests/adversary2.rs
?? crates/eval/tests/fixtures/adversary2/
```

**2. Cases added**

All 15 are in `crates/eval/tests/adversary2.rs`, with fixtures under `crates/eval/tests/fixtures/adversary2/<pkg>/`. 11 are red and 4 are green; the green ones are the `adv2_mut_*` cases, which each catch a mutant the existing suite misses.

I ran each case alone first (`cargo test -p b10x-codespec-eval --test adversary2`). Two cases failed that first run for reasons in my own test code, not the evaluator:
- The wide-type case's JSON lacked the `reader` field. Once fixed it passed, so I removed it (listed in part 5).
- `U` returned `uint8`, which the kit's driver refuses to print. I changed it to return `bool`, and it is now a green mutant catcher.

Go side: go1.27.1, `scratch/adv2/gorun` imports the same fixtures:
```
vari.Same: false            vari.Text: "A!"   vari.Closure: false   vari.Spread: false   vari.Ret: false
errkey.ErrKeys: 1           errkey.ErrLit: true
mapsnap.Sum: 11             (100 runs: 100× 11)
nilrecv.NilRecv: "nil"      initfn.X: 5       pcts.Pct: "S!"        rangeassign.Last: 2
strrange.First: 0  arrsum.Sum: 0  deep.Deep: 24750   (all three in 93 ms under a 3 GB address-space cap)
ifacemut.{Left,Tag,Read,Lookup,Global,Bools,Appended}: true "one" 5 5 true true true
limits.Big: 4194304         limits.U(255): true
bldcopy.Copy: panic: strings: illegal use of non-zero Builder copied by value
```

Red output from the solo runs, verbatim:
```
adv2_values_reaching_any_by_variadic_closure_spread_or_tuple_keep_their_type
  left: "ok true\nok \"1\"\nok true\nok true\nok true\n"   right: "ok false\nok \"A!\"\nok false\nok false\nok false\n"
adv2_an_error_value_and_its_concrete_value_are_equal_as_any
  left: "ok 2\nok false\n"   right: "ok 1\nok true\n"
adv2_an_interface_holding_a_nil_pointer_passes_nil_to_the_method
  left: "ok \"t\"\n"   right: "ok \"nil\"\n"
adv2_a_map_range_reads_each_value_when_it_reaches_it      left: "ok 2\n"  right: "ok 11\n"
adv2_init_functions_run_before_the_calls                  left: "ok 0\n"  right: "ok 5\n"
adv2_percent_s_calls_string_on_a_string_type              left: "ok \"raw\"\n"  right: "ok \"S!\"\n"
adv2_a_range_with_assign_sets_the_variable_declared_before_it  left: "ok 0\n"  right: "ok 2\n"
adv2_a_write_to_a_copied_builder_stops                    Go panics, the evaluator printed "ok \"xy\"\n"
adv2_a_range_over_a_long_string_builds_no_list            status Some(ExitStatus(unix_wait_status(6))): memory allocation of 2147483648 bytes failed
adv2_a_step_budget_bounds_the_time_a_run_takes            1000 steps did not end within 10 s (status None)
adv2_calls_nested_within_the_stated_limit_do_not_overflow_the_stack
  status Some(ExitStatus(unix_wait_status(6))): thread 'cir1-eval' (1238635) has overflowed its stack / fatal runtime error: stack overflow, aborting
```

The same failures through the CLI (`target/debug/codespec eval`):
- `deep` exits 134. The overflow already happens with 16 additions at a call depth of 990; 12 additions run fine.
- `strrange` exits 134 under a 3 GB cap.
- `arrsum` with `--max-steps 1000` takes 32 s before it stops at the step budget.

**3. Suite run**, after the cases existed: `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo test --locked --no-fail-fast -p b10x-codespec-eval -p b10x-codespec-cli`, exit 101.
```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.44s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 14 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.34s
test result: FAILED. 4 passed; 11 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s
error: test failed, to rerun pass `-p b10x-codespec-eval --test adversary2`
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.87s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.67s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
error: 1 target failed:
```
The "67" in the header is this run's 82 minus the 15 that the `adversary2` binary ran. It is not a separate run with my file deselected. The kit check (`tests/kit.rs`, the same test `task eval-kit` runs) passed: 2 of 2.

**4. Findings**

| # | sev | file:line | test | verdict / origin | what reaches it |
|---|---|---|---|---|---|
| F1 | major | `crates/eval/src/interp.rs:783` (also :751, :1841, :1069) | `adv2_values_reaching_any_by_variadic_closure_spread_or_tuple_keep_their_type` | NEEDS-CHANGE / introduced | Pass-1 F2 again. A value passed through a variadic `...any`, a function-literal or function-value call, a spread two-valued call, or a returned tuple is never converted to the interface. `==`, map keys and `String()` then give wrong results silently. Reached by the Go reader on ordinary code. |
| F2 | major | `crates/eval/src/interp.rs:2787` | `adv2_an_error_value_and_its_concrete_value_are_equal_as_any` | NEEDS-CHANGE / introduced | Pass-1 F2 again. `equal` treats a value held as an `error` and the same value held as `any` as different. Wrong map length and wrong `==`. Reached by the Go reader, which writes `to_error` for a type that has an `Error` method. |
| F3 | major | `crates/eval/src/interp.rs:856` | `adv2_an_interface_holding_a_nil_pointer_passes_nil_to_the_method` | NEEDS-CHANGE / introduced | A method called through an interface holding a nil pointer gets a pointer to a new cell holding nil, so the method's `t == nil` check is false. Common nil-receiver idiom; Go reader. |
| F4 | major | `crates/eval/src/interp.rs:1245` | `adv2_a_map_range_reads_each_value_when_it_reaches_it` | NEEDS-CHANGE / introduced | A range over a map reads a copy of the entries taken before the loop. Go reads each value when the loop reaches it; Go's output is 11 in 100 of 100 runs. Go reader. |
| F5 | major | `crates/eval/src/interp.rs:2514` | `adv2_init_functions_run_before_the_calls` | NEEDS-CHANGE / introduced | `func init()` is never run (the reader writes it as a plain function named `init`), so the run prints zero values. Go reader. |
| F6 | minor | `crates/eval/src/interp.rs:2299` | `adv2_percent_s_calls_string_on_a_string_type` | NEEDS-CHANGE / introduced | `%s` on a string type with a `String` method prints the raw string; `%v` already calls the method. |
| F7 | minor | `crates/eval/src/builtins.rs:79` | `adv2_a_write_to_a_copied_builder_stops` | NEEDS-CHANGE / introduced | Go panics on a write to a copied non-zero `strings.Builder`; the handler writes and prints a value. |
| F8 | major | `crates/eval/src/interp.rs:1567` (and :1531) | `adv2_a_step_budget_bounds_the_time_a_run_takes` | NEEDS-CHANGE / introduced | Pass-1 F3 class. Each `a[i]` or `len(a)` copies the whole array, so a step costs O(array size). A 1000-step budget took 32 s; the default 10M budget would take about 90 h on a loop Go finishes in under 100 ms. |
| F9 | minor | `crates/eval/src/interp.rs:1250` | `adv2_a_range_over_a_long_string_builds_no_list` | NEEDS-CHANGE / introduced | Pass-1 F4 class. A range over a string builds its full rune list first (16 bytes per byte), and the allocation aborts the process. |
| F10 | major | `crates/eval/src/lib.rs:155` | `adv2_calls_nested_within_the_stated_limit_do_not_overflow_the_stack` | NEEDS-CHANGE / introduced | Pass-1 F10 class. The 512 MiB stack fits 1000 nested calls but not expression depth: `f(n-1) + 1 …` with 16 additions at a call depth of 990 overflows (exit 134). Valid Go, read by the Go reader, within the stated nesting limit of 1000. |
| F11 | minor | `adapters/go/cmd/codespec-read-go/front.go:844` | `adv2_a_range_with_assign_sets_the_variable_declared_before_it` | CONFIRMED / pre-existing | The reader ignores `s.Tok`, so `for i = range` is written as `for i := range`, and the evaluator correctly leaves the outer `i` unchanged. Same code at base d614aaf. A non-identifier key (`for s.f = range`) would also panic the reader at :839. |
| F12 | minor | `crates/eval/tests/adversary1.rs:158` | `adv2_mut_*` (green) | CONFIRMED / introduced | Gap in the existing tests: 9 mutants survive the suite (part 7). The pass-1 `Held` case expects `false`, which an append with no conversion also gives, so it cannot catch M13. |
| F13 | note | `crates/eval/src/check.rs:96` | none (mutant M3) | CONFIRMED / introduced | The bool refusal can never fire: the `check_argument` call inside `calls_go` already refuses anything but `true` or `false`. |
| F14 | minor | `website/data/status.json:208` | none | CONFIRMED / introduced | Pass-1 F16 is not closed. `d614aaf..HEAD` still changes `website/data/status.json`, `website/docs/reference/{cli,crates}.md` and `crates/docs/src/status.rs`, and d0523cc adds 6 more lines to `cli.md`. |

**Pass-1 findings**
- F1: not closed. It is the accepted stated limit, and its test is `#[ignore]`.
- F2: not closed (F1, F2 and F3 above).
- F3: closed for `for {}`; F8 is a neighbour in the same class.
- F4: closed for range over an int; F9 is the string-shaped neighbour.
- F5, F6, F7: closed.
- F8: closed. The heap growth code matches go1.27.1 `growslice`/`roundupsize`, read line by line.
- F9: closed, including dependencies through a function literal (probe printed 3, as Go does). F5 above is a separate initialisation defect.
- F10: closed for self-holding types; F10 above is a new neighbour.
- F11, F12, F13, F14: closed.
- F15: accepted.
- F16: not closed (F14 above).

**Mutants.** Each was applied to a copy of the crate in scratch, built into the worktree's `target`, then run against the existing suite (lib, adversary1, ids, kit, refusals, stops) and then my `adv2_mut_*` cases.

| id | mutation | existing suite | adv2_mut |
|---|---|---|---|
| M1 | check.rs:183 `>` → `>=` | survived | caught (`…zero_value_at_the_stated_limit_runs`) |
| M2 | check.rs:88 width check treats unsigned as signed | survived | caught (`…negative_argument…unsigned…`) |
| M3 | check.rs:96 bool refusal removed | survived | survived (dead code, F13) |
| M5 | interp.rs:1696 left operand of `==` not converted | survived | caught |
| M6 | interp.rs:1052 switch tag not converted | survived | caught |
| M7 | interp.rs:1571 map-read key not converted | survived | caught |
| M8 | interp.rs:1100 `lookup_else` key not converted | survived | caught (`…lookup_else_converts_its_key`; the case is hand-built because the Go reader never writes `lookup_else`) |
| M9 | interp.rs:2558 package variable not converted | survived | caught |
| M10 | interp.rs:413 bool laid out as 8 bytes | survived | caught (`Bools`) |
| M13 | interp.rs:2112 appended value not converted | survived | caught (`Appended`) |
| M15 | growth.rs:85 `>` → `>=` | caught (`capacities_are_gos`) | n/a |

I reasoned through, but did not build, three more:
- `if stack.contains(&key)` disabled in check.rs: still refused, by the depth limit.
- `!self.is_iface(tn)` dropped from the check: equivalent, because interface calls are `IfaceCall`.
- `len > 0` dropped from `Layout::array`: only differs for zero-length arrays of pointers.

**5. Attacked and could not break**
- Refusing a call into unreached code that the reader marks foreign: the reader's foreign/opaque output already prevents it.
- Exponential type fan-out in `check::cells`: it stops at the first type over the limit. Go's own type checker takes 80 s on 26 levels.
- Growth constants against go1.27.1 source.
- Calls-file escapes, `"010"`, short strings: `check_argument` refuses them.
- `make` with a length or capacity, `break` inside a switch, local types, `complex128`: all foreign stops.
- `String`/`Error` on nil pointers with pointer receivers via `fmt`.
- Dependency order through closures and method values.

No lease was taken explicitly; host hooks may have taken one.

**6. Paths written outside the worktree**
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/cs/eval-core/adv2/`: probe Go module, `gorun` Go program and binaries, `mut/` mutant workspace (`Cargo.toml`, `Cargo.lock`, copied crate, symlinks to the worktree's `conformance` and `adapters`), scripts, logs.

Inside the worktree's `target/` I added `tmp/eval-reader-adv2/` and the mutant copy's artifacts, built with stable rustc 1.99.1 instead of the pinned 1.97. No core dumps were stored.

**7. Findings block**
```findings
- file: crates/eval/src/interp.rs
  line: 783
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: values reaching any through a variadic, a function value call, a spread tuple or a tuple return keep no dynamic type, so ==, map keys and String give Go-different answers without a stop
- file: crates/eval/src/interp.rs
  line: 2787
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: equal treats a value held as an error and the same value held as any as different
- file: crates/eval/src/interp.rs
  line: 856
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a pointer-receiver method called through an interface holding a nil pointer receives a non-nil pointer to a new cell
- file: crates/eval/src/interp.rs
  line: 1245
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a range over a map iterates a copy taken before the loop, so values changed during the loop are read stale
- file: crates/eval/src/interp.rs
  line: 2514
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: package init functions are never run and the run prints zero values without a stop
- file: crates/eval/src/interp.rs
  line: 2299
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Sprintf %s of a string type with a String method prints the raw string
- file: crates/eval/src/builtins.rs
  line: 79
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a write to a copied non-zero strings.Builder succeeds where Go panics
- file: crates/eval/src/interp.rs
  line: 1567
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: indexing or taking len of an array copies the whole array per step, so the step budget does not bound run time
- file: crates/eval/src/interp.rs
  line: 1250
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a range over a string builds its full rune list before the first iteration and aborts on a long string
- file: crates/eval/src/lib.rs
  line: 155
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: calls nested within the stated 1000 through an expression of 16 or more additions overflow the 512 MiB stack and abort
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 844
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the Go reader writes for i = range as for_range, which declares i, so the outer variable is never assigned
- file: crates/eval/tests/adversary1.rs
  line: 158
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: nine mutants of check.rs and the interface conversions survive the suite, and the Held case cannot fail for an unconverted append
- file: crates/eval/src/check.rs
  line: 96
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the bool argument refusal is unreachable because calls_go refuses non-bool literals first
- file: website/data/status.json
  line: 208
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: pass-1 F16 stands, generated site files and the docs status entry are still in the unit's diff and d0523cc adds to cli.md
```


Coordinator note: in the findings block above `major` is written as `warning` and `minor` as `note`, the severities the store reads; the table in part 4 has the reviewer's words. Nothing else is changed.
