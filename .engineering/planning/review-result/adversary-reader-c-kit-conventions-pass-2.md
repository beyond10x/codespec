---
format: aep.planning-md/3
id: review-result:adversary-reader-c-kit-conventions-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the C reader's conventions unit
relations:
- reviews: story:reader-c-kit-conventions
revision: 1
---
unit: `story:reader-c-kit-conventions`, HEAD `bf89e62` (base `e6c404b`) in `/home/timo/.local/state/worktree/trees/b10x/codespec/rd-c-conv`, plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 120→148, red 9
origin: introduced 8 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 directory (1.3 GB), listed in part 6
needs-coordinator: none. I took no session lease and changed no git state.

**Verdict: NEEDS-CHANGE.** I found 7 C programs that the reader reads with no refusal into code that gives a different result from the C. 6 come from this unit; 1 (H3) was already in the base. There are also 2 more findings: H8 (a pre-existing case where the C order is open), and the 19 mutants that change meaning and still pass the suite.

**1. Diff stat.** `git --no-pager diff --stat` is empty. The only change is the new untracked file `adapters/c-read/tests/adversary3.rs`. It is not committed.

**2. Cases added** in `/home/timo/.local/state/worktree/trees/b10x/codespec/rd-c-conv/adapters/c-read/tests/adversary3.rs`: 28 tests, 9 red and 19 green.
- **The 9 red cases.** Each was red the first time it ran alone (`cargo test -p codespec-read-c --test adversary3`). "C" is what `codespec-run-c` prints. "Go" is the read put through `codespec pass` and `write --lang go`, then run by `codespec-run-go`.
- **The 19 green `kill_mNN_*` cases.** Each one is green at HEAD and goes red under its own mutant, and under no other mutant. I checked this in a scratch copy of the tree; the tree under attack was not touched.

**3. Suite run, after the cases existed.** Command: `cargo test --locked --no-fail-fast -p codespec-read-c -p codespec-run-c`. Exit 101.
- Passed: adversary1 5, adversary2 32, cases 3, reader 16, runtime 54, `codespec_run_c` lib 3, acceptance 7.
- Failed: adversary3, 19 passed and 9 failed.
- "Before" is 120: the same run without adversary3.

**4. Findings** (they cover `bf89e62` plus the untracked file)

| id | sev | file:line | origin | verdict | what |
|---|---|---|---|---|---|
| H1 | high | rtbody.rs:1783 | introduced | NEEDS-CHANGE | `order_free` matches accumulators by name only. With `pa = &a`, the body `a += v; b += *pa` passes the check. C gives `ok 35` every time; Go gives 35, 45 or 50. |
| H2 | high | body.rs:168 | introduced | NEEDS-CHANGE | `has_call` does not count `append`. `Two(cs_append(s,…2), cs_append(s,…3))` with spare capacity gives C `ok 22` and Go `ok 33`. The C writer's `has_call` has the same gap, so the writer emits this shape too. |
| H4 | high | lib.rs:295 | introduced | NEEDS-CHANGE | `unprefix` renames `m_Twice` to `Twice` inside a function that has a local `Twice`. The call then reaches the local: C `ok 5020`, Go `ok 5005`. |
| H5 | high | lib.rs:295 | introduced | NEEDS-CHANGE | `m_twice` and `twice` both read as `twice`, and `codespec validate` accepts the module. C `ok 20030`, Go `ok 30030`. |
| H7 | high | rtbody.rs:1049 | introduced | NEEDS-CHANGE | The F4 fix does not see a pointer target. `*p = cs_t1; a[i] = cs_t2;` with `p = &i` gives C `ok 0` and Go `ok 7`. |
| H6 | medium | body.rs:189 | introduced | NEEDS-CHANGE | `cs_div`, `cs_slice_at` and `cs_shift_count` are C calls that can panic, but `ordered` does not count them. `Two(Tick(1), cs_div_int64_t(10, d))`: C panics without printing; Go prints `1` and then panics. |
| H9 | low | body.rs:1339 | introduced | NEEDS-CHANGE | A call of a function without a `codespec: func` line checks no argument types. An `int64` reaches an `int` parameter; `codespec pass` accepts it and the Go does not compile. |
| H3 | medium | body.rs:1225 | pre-existing | CONFIRMED | A plain `x << n` with `n` = 64 reads as Go `<<`. C (x86) gives `ok 1`, Go `ok 0`. The base reader reads it the same way. |
| H8 | medium | body.rs:770 | pre-existing | CONFIRMED | `a[k] = bump()` where `bump` writes `k`: C gives `ok 70`, Go gives `ok 7`. The base reader reads it the same way. `ordered` only counts calls. |
| H10 | medium | tests/runtime.rs | introduced | NEEDS-CHANGE | 27 of 28 mutants of the new checks survive the suite; only M14b is caught. Of the survivors, 19 change meaning, and adversary3 now has a kill case for each. They are: ordered's Send, LookupElse and single-target Assign arms; order_free's call check, accumulator-read check, one-statement-per-accumulator check and integer-only check; every near-miss check of the range block (`++`, `<`, start at 0, bound, element type, key type, map counter `int64_t`, entry callee, `order_free`); the scoped `for`'s empty-init check; multi-assign's call-target check; and the return-type check. |
| H11 | low | rtbody.rs:822 | introduced | CONFIRMED | The check against a send on a receive-only channel can never run: a `codespec: func` line cannot spell `<-chan int` (the line itself is refused), so mutant M20 is dead code. |

What reaches these: hand-written C in runtime form, which is the input of the reader corpus. For H4 and H5, any directory with one header holding `codespec:` lines.

Mutants I judged equivalent:
- M05 and M16.
- M15: `value()` already refuses the implicit conversion.
- M34: one declaration types both counters.
- M38: a later type check catches it.
- M02: gcc evaluates initializers in order.
- M09 (`-` added to the accumulator operators) changes meaning only through `s = v - s`. I wrote no kill case for it.
- One caveat: in the main mutant run, every mutant after M02 also had the M03 mutation applied, because an interrupted earlier run left the scratch copy mutated. The kill checks ran on a clean copy.

Fixes, named and not applied:
- H1, H7: treat a deref or a read of memory as touching every local whose address is taken.
- H2, H6: count `append` and the runtime calls that can panic in `has_call`, in the reader and in the writer.
- H4, H5: refuse when stripping the prefix makes a name collide with a declaration or a local, or keep the prefix.
- H9: type the arguments of functions without a line from their C signature.

**5. What held**
- Width and signedness:
  - `char` is refused.
  - Integer promotion of `uint8_t`/`uint16_t` in `+=` and `*=` is refused.
  - Mixed signed/unsigned comparisons and constants that do not fit are refused.
  - The wrap-form constants are refused.
- Shift checks: the `cs_shl`/`cs_shr` count checks hold.
- `(void)` statements: `(void)cs_map_put` and `(void)cs_map_get` are refused.
- Slice range through a pointer: a `cs_slice *` alias is refused.
- Ordering cases where C and Go agree:
  - `n + Bump()`: C and Go agree (106 and 105).
  - `Two(10/d, Tick(1))`: both print `1`, then panic.
- Runner: on the F10 runner (one compile per file, functions declared from their lines), the line and the definition are held together by the reader's `agrees` check, and I found no mismatch the reader lets through.
- Append capacity: an aliasing test of capacity gave 33 in both, since Go 1.25+ gives small appends a capacity of 4.

**6. Paths written outside the worktree**
- `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/c-adv2/`, 1.3 GB:
  - `probe.sh`, `goenv`, `p/*` (probes), `base/` (an archive of the base `e6c404b` with the reader built in it)
  - `mut/tree`, `mut/tree2` (copies of HEAD with their own `target/`), `mut/mutate.py`, `mut/all.sh`, `mut/kill.sh`, `mut/*.log`, `mut/kill/`
  - `gocache/`, `gotmp/`, `*.log`
- Inside the tree, untracked: `target/c-read-adversary3/` (940 KB). The worktree's `target/debug` was rebuilt by cargo.

**7. Findings block**
```findings
- file: adapters/c-read/src/rtbody.rs
  line: 1783
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H1: order_free matches accumulators by name, so b += *pa with pa = &a reads as an unordered range though the result depends on order (C 35 always, Go 35/45/50) (case: a_map_loop_whose_accumulator_reads_another_through_a_pointer_is_refused)"
- file: adapters/c-read/src/body.rs
  line: 168
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H2: has_call does not count append, so two cs_append of one slice as call arguments read left to right while gcc goes right to left (C 22, Go 33); the C writer's has_call has the same gap (case: two_appends_to_one_slice_as_arguments_of_one_call_are_refused)"
- file: adapters/c-read/src/lib.rs
  line: 295
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H4: unprefix renames m_Twice to Twice inside a function that has a local Twice, so the Go call reaches the local (C 5020, Go 5005) (case: a_local_named_as_a_module_function_without_its_prefix_does_not_capture_its_calls)"
- file: adapters/c-read/src/lib.rs
  line: 295
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H5: unprefix reads m_twice and twice as two functions named twice, which codespec validate accepts (C 20030, Go 30030) (case: two_functions_that_differ_only_by_the_module_prefix_do_not_read_to_one_name)"
- file: adapters/c-read/src/rtbody.rs
  line: 1049
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H7: the later-target check compares by name, so *p = cs_t1; a[i] = cs_t2 with p = &i reads as *p, a[i] = 1, v (C 0, Go 7) (case: a_multiple_assignment_whose_later_index_an_earlier_pointer_target_writes_is_refused)"
- file: adapters/c-read/src/body.rs
  line: 189
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H6: ordered does not count runtime calls that can panic (cs_div, cs_slice_at, cs_shift_count): Two(Tick(1), cs_div(10, 0)) panics silently in C and prints 1 first in Go (case: a_call_beside_a_runtime_division_that_panics_is_refused)"
- file: adapters/c-read/src/body.rs
  line: 1339
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H9: a call of a function without a codespec: func line checks no argument type, so an int64 reaches an int parameter and the Go does not compile (case: a_call_of_a_function_without_a_line_whose_argument_has_another_type_is_refused)"
- file: adapters/c-read/src/body.rs
  line: 1225
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "H3: a plain x << n with n of 64 or more reads as Go's shift (0) while C (x86) gives x << (n & 63) (1) (case: a_plain_shift_by_a_count_the_reader_cannot_bound_is_refused)"
- file: adapters/c-read/src/body.rs
  line: 770
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "H8: a[k] = bump() where bump writes k reads without refusal; gcc indexes with the old k (70), Go with the new (7); ordered only counts calls (case: an_assignment_whose_index_the_call_on_the_right_writes_is_refused)"
- file: adapters/c-read/tests/runtime.rs
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "H10: 27 of 28 mutants of the new checks survive the suite, 19 of them meaning-changing; each now has a kill_mNN case in adversary3.rs, red under its mutant (case: kill_m03_a_send_whose_channel_and_value_both_make_calls_is_refused)"
- file: adapters/c-read/src/rtbody.rs
  line: 822
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "H11: the send-on-receive-only check can never run: a codespec: func line cannot spell <-chan int, so mutant M20 is dead code (case: judgement)"
```

adversary3 test lines, verbatim from the suite run:
```
test a_plain_shift_by_a_count_the_reader_cannot_bound_is_refused ... FAILED
test an_assignment_whose_index_the_call_on_the_right_writes_is_refused ... FAILED
test a_map_loop_whose_accumulator_reads_another_through_a_pointer_is_refused ... FAILED
test kill_m04b_an_assignment_whose_index_and_value_both_make_calls_is_refused ... ok
test kill_m06_a_map_loop_whose_accumulated_part_makes_a_call_is_refused ... ok
test kill_m32_a_range_block_starting_at_one_is_refused ... ok
test kill_m07_a_map_loop_whose_part_reads_another_accumulator_is_refused ... ok
test kill_m24_a_multiple_assignment_to_a_target_that_makes_a_call_is_refused ... ok
test kill_m09c_a_map_loop_accumulating_a_float_is_refused ... ok
test kill_m27_a_block_whose_for_has_an_init_of_its_own_is_refused ... ok
test kill_m03_a_send_whose_channel_and_value_both_make_calls_is_refused ... ok
test kill_m12_a_map_range_block_with_an_int8_t_counter_is_refused ... ok
test a_call_beside_a_runtime_division_that_panics_is_refused ... FAILED
test kill_m36_a_map_range_block_reading_the_value_first_binds_the_value ... ok
test kill_m47_a_range_block_whose_key_is_narrower_than_the_index_is_refused ... ok
test kill_m04_a_lookup_whose_map_and_key_both_make_calls_is_refused ... ok
test kill_m11_a_range_block_up_to_and_including_its_length_is_refused ... ok
test a_multiple_assignment_whose_later_index_an_earlier_pointer_target_writes_is_refused ... FAILED
test a_call_of_a_function_without_a_line_whose_argument_has_another_type_is_refused ... FAILED
test kill_m22_a_return_of_another_type_than_the_line_s_result_is_refused ... ok
test kill_m10_a_range_block_counting_down_is_refused ... ok
test kill_m14_a_map_range_block_whose_body_depends_on_the_order_is_refused ... ok
test a_local_named_as_a_module_function_without_its_prefix_does_not_capture_its_calls ... FAILED
test kill_m08_a_map_loop_with_two_statements_on_one_accumulator_is_refused ... ok
test two_appends_to_one_slice_as_arguments_of_one_call_are_refused ... FAILED
test two_functions_that_differ_only_by_the_module_prefix_do_not_read_to_one_name ... FAILED
test kill_m33_a_range_block_bounded_by_a_constant_is_refused ... ok
test kill_m35_a_range_block_reading_its_element_as_another_type_is_refused ... ok
test result: FAILED. 19 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.27s
```

`git status --short`:
```
?? adapters/c-read/tests/adversary3.rs
```
