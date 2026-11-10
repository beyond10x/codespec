---
format: aep.planning-md/3
id: review-result:adversary-reader-c-kit-conventions-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the C reader's runtime-form read
relations:
- reviews: story:reader-c-kit-conventions
revision: 1
---
unit: `story:reader-c-kit-conventions`, HEAD `44f8166` (base `e6c404b`), tree `/home/timo/.local/state/worktree/trees/b10x/codespec/rd-c-conv` plus one new untracked file
verdict: NEEDS-CHANGE
cases: executed 73→105, red 14
origin: introduced 12 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (467 MB of scratch, listed in part 6)
needs-coordinator: none. I did not take a session lease on the tree; I changed no git state.

**1. Diff stat**
`git --no-pager diff --stat` is empty: the only change is a new untracked test file.
```
?? adapters/c-read/tests/adversary2.rs
```
No fixture directory was needed; every C source is inline in the test.

**2. Cases added** (`adapters/c-read/tests/adversary2.rs`, 32 tests: 14 red, 18 green)

The red ones. Each was red the first time anything ran it, alone (`cargo test -p codespec-read-c --test adversary2`):
```
order-println: read without a refusal at case.c:11, though C prints 2 before 1 and the read println(Tick(1), Tick(2)) prints 1 before 2; refusals: []
order-cat: read without a refusal at case.c:11, though C prints b before a and the read Tag("a") + Tag("b") prints a before b; refusals: []
order-div: read without a refusal at case.c:11, though C prints 4 before 8 and the read Tick(8) / Tick(4) prints 8 before 4; refusals: []
order-plain: read without a refusal at case.c:19, though C returns 21002 and the read pair(g(), h()) returns 1012; refusals: []
range-int8: read without a refusal at case.c:10, though C panics at index -128 and the read range over xs does not; refusals: []
multi-assign-index: read without a refusal at case.c:7, though C assigns a[1] and the read i, a[i] = 1, v assigns a[0]; refusals: []
ill-typed-arg: read without a refusal at case.c:18, though the read passes a []uint64 to a []int parameter; refusals: []
recv-send-only: read without a refusal at case.c:5, though the read receives from a send-only channel; refusals: []
map-range-alias: read without a refusal at case.c:11, though C visits the keys the body inserts and Go's range may skip them; refusals: []
map-range-order: read without a refusal at case.c:10, though C returns the first key inserted and the read returns any key; refusals: []
the_runner_and_the_reader_read_the_same_program_...: left: "ok 3\n" right: "ok 10\n"
map-get-assign: read without a refusal at case.c:6, though C writes a value outside the map and the read m[k] = 5 inserts k; refusals: []
a_function_of_int64_as_the_c_writer_writes_it_...: "case.c:4 function Wide: its codespec: func line: parameter x: its C type int64_t does not hold the codespec: line's type"
an_index_loop_over_a_slice_reads_as_a_for_loop: "case.c:6 for (int64_t i = 0; i < xs.len; i++) {: a loop over a slice other than the form the reader reads as range"
```
The 18 green ones each kill one surviving mutant (part 4, F13). Each turns red under its own mutant and under no other: `adv1/mut/kill.sh` gives 15 failures per mutant against 14 for the unmutated build.

**3. Suite run** (after the cases existed)
`CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 RUSTC_WRAPPER= cargo test --locked --no-fail-fast -p codespec-read-c -p codespec-run-c`: exit 101.
```
adversary1  ok. 5 passed
adversary2  FAILED. 18 passed; 14 failed
cases       ok. 3 passed
reader      ok. 16 passed
runtime     ok. 39 passed
codespec_run_c lib ok. 3 passed
acceptance  ok. 7 passed
```
"Before" is the same run with `adversary2` left out: 73.

**4. Findings**

All findings cover HEAD `44f8166` plus the untracked test file. "C" means the C compiled with the kit header and run by `codespec-run-c`. "Go" means the read passed through `codespec pass`, written as Go and run by `codespec-run-go`.

| # | sev | file:line | origin | verdict | finding | test |
|---|---|---|---|---|---|---|
| F1 | blocker | adapters/c-read/src/rtbody.rs:596 (builder chains; also `two` at :414 for `cs_string_eq/cat` and `cs_div/rem`) | introduced | NEEDS-CHANGE | C leaves the order of a call's arguments unspecified, and gcc evaluates the outer argument first. `cs_sb_println(cs_fmt_int(cs_sb_bytes(cs_fmt_int(cs_sb_new(), Tick(1)), " ", 1), Tick(2)))`: C prints `2,1,"1 2"`, the read `println(Tick(1), Tick(2))` prints `1,2,"1 2"`. Same for `cs_string_cat` (b, a against a, b) and `cs_div_int64_t` (4, 8 against 8, 4). The C writer refuses this shape (`write.rs` `order`). | `a_printed_line_of_two_calls…`, `cs_string_cat_of_two_calls…`, `cs_div_of_two_calls…` |
| F2 | blocker | adapters/c-read/src/body.rs:1218 | pre-existing (the base `body.rs:1141` has the same loop and no order check) | NEEDS-CHANGE | The same in plain C: `pair(g(), h())` gives C `ok 21002`, Go `ok 1012`. | `a_plain_call_of_two_calls…` |
| F3 | blocker | adapters/c-read/src/rtbody.rs:1074 | introduced | NEEDS-CHANGE | `rt_range` does not check the counter's type. With `for (int8_t j = 0; j < xs.len; j++)`, `Total(200)` panics in C (exit 3, index -128) and returns `ok 19900` in Go. | `a_loop_over_a_slice_whose_counter_is_narrower…` |
| F4 | blocker | adapters/c-read/src/rtbody.rs:858 | introduced | NEEDS-CHANGE | `i = cs_t1; a[i] = cs_t2;` is read as `i, a[i] = 1, v`. Go indexes with the old `i`, C with the new one: `Pick(7)` gives C `ok 0`, Go `ok 7`. | `a_multiple_assignment_whose_later_target…` |
| F5 | blocker | adapters/c-read/src/rtbody.rs:337 (reached through `place` at body.rs:744) | introduced | NEEDS-CHANGE | `*(V *)cs_map_get(m,&k,…) = 5;` and `(*(V *)cs_map_get(…))++` are read as `m[k] = 5` and `m[k]++`. For a missing key `cs_map_get` gives a fresh allocation, so C does not insert the key: C `ok 0, ok 0`, Go `ok 1, ok 1`. | `an_assignment_or_increment_through_cs_map_get…` |
| F6 | warning | adapters/c-read/src/body.rs:1207 | introduced | NEEDS-CHANGE | Calls typed by `codespec: func` lines do not check argument (or return) types: a `[]uint64` passed to a `[]int` parameter is written without a refusal. C prints `ok -1`; `codespec pass` accepts the module; Go does not compile. | `a_slice_passed_to_a_parameter…` |
| F7 | warning | adapters/c-read/src/rtbody.rs:355 | introduced | NEEDS-CHANGE | `cs_chan_recv` on a channel whose line makes it `chan<- int` is read as `<-ch`, which is ill-typed. | `a_receive_from_a_send_only_channel…` |
| F8 | warning | adapters/c-read/src/rtbody.rs:1166 | introduced | CONFIRMED | `written` sees only `cs_map_put(m, …)` on the loop's own name. Inserting through `alias = m` gives C `ok 6`, Go `ok 5`. Go allows either result, but the crate doc's own rule (refuse a range whose body puts into the map) is bypassed. | `a_loop_over_a_map_whose_body_puts_into_it_through_an_alias…` |
| F9 | warning | adapters/c-read/src/rtbody.rs:1086 | introduced | CONFIRMED | The runtime walks a map in insertion order and `range m` has no order. An order-dependent body: C prints `ok 0` ×5, Go prints `0,0,1,0,4`. Whether a deterministic C loop may be read as an unordered range is a design question for the story. | `a_loop_over_a_map_whose_body_depends_on_the_order…` |
| F10 | warning | adapters/c-run/src/lib.rs:499 | introduced | CONFIRMED | In runtime form the runner `#include`s every `.c` into one translation unit, while the reader reads each file alone. A `#define LIMIT 3` in `a.c` reaches `b.c` in the runner (`ok 3`), not in the reader (`return 10`). What reaches it: any multi-file hand-written runtime-form directory; no kit case has one. | `the_runner_and_the_reader_read_the_same_program…` |
| F11 | warning | adapters/c-read/src/runtime.rs:100 | introduced | NEEDS-CHANGE | `int64_t` is always read as `int`, so the C writer's own `/* codespec: func Wide(int64) int64 */ int64_t Wide(int64_t)` is refused. `uint` over `uint64_t` is refused the same way. This is an over-refusal in the round trip. | `a_function_of_int64_as_the_c_writer_writes_it…` |
| F12 | warning | adapters/c-read/src/rtbody.rs:1159 | introduced | NEEDS-CHANGE | `rt_range` claims every loop `for (int64_t i = 0; i < xs.len; i++)` and refuses it when the body does not start with the element read. It should give the loop back to the plain `for` reading, which is exact here. | `an_index_loop_over_a_slice_reads_as_a_for_loop` |
| F13 | warning | adapters/c-read/tests/runtime.rs | introduced | NEEDS-CHANGE | Details below the table. | the 18 green `kill-*` / `a_*` cases |

F13: 20 of 25 side-condition mutants of `rtbody.rs` and `runtime.rs` survive the suite; 18 of them change meaning. The suite misses:
- step other than `++`
- element read from another slice or index
- map key read as another type
- wrap operand of another type
- `/` read as a wrapping form
- `else` after the lookup miss
- tuple binding to a narrower variable
- multi-assign temporaries of another type
- switch case of another type than its tag
- `.cap` read as `len`
- `CS_KEY_U32` swapped for `CS_KEY_I32`
- a duplicate `codespec: func` line
- the `RT_FILE` check on `rt_callee`
- `<=` read as range
- the parameter count check
- any global named `Err…` read as a sentinel
- `cs_fmt_int` of an unsigned value
- `sorted_keys` of bool keys

m10 (the miss branch using `p`) and m15 (`&` in `written`) survive too; I judge them equivalent, because the reader refuses that use elsewhere. Killed: m01, m02, m08, m09, m24. The acceptance asks for "one near-miss test per recognised form"; none exists for `cs_string_eq/cat`, `cs_div/rem`, `cs_map_len`, or `return (<Name>_result){…}`.

Fixes, named and not applied:
- F1/F2: refuse a call or operator with more than one argument containing a call, as `write.rs` `order` does.
- F3: require the counter's type to be `int64_t`.
- F4: refuse when a later target refers to an earlier target.
- F5: in `place`, refuse `rt_deref` of `cs_map_get` and of `cs_chan_recv`.
- F6/F7: check argument and return types against the signature, and check the channel direction.
- F12: return `None` instead of `Err` when no element read is taken.

Mutants were compiled with `rustup run 1.97 rustc` against the tree's dependency rlibs, read-only, into scratch. Each was run by bind-mounting it over `target/debug/codespec-read-c` inside `bwrap`; the file on disk was never touched.

**5. Areas that held**
- Kit header equality: `kit == writer` is a byte comparison, so it goes red on any change to either file. I read this; I did not run a mutant on each side.
- Determinism: the 27 runtime-form kit cases, each read twice, are byte-equal.
- String literals: an embedded NUL and `\x` escapes are refused; a concatenated literal reads correctly; `%` in `cs_sb_bytes` becomes `%%` (`"100%d 5"` matches).
- Wrap: `(int64_t)(0UL - (uint64_t)a)` and a mixed-width operand are refused.
- `&*cs_map_get(…)` is refused.
- A `break` in an if-chain switch inside a plain C `switch` is refused (body.rs:448).
- Returning `(Two_result){Tick(1), Tick(2)}`: gcc evaluates the initializers in order, and C matches Go.
- Pointer escape from a lookup, a lookup miss that does not return, and a temporary used later are all refused (existing tests).
- `codespec-run-c`: two sources with the same `static` name, or a source with `main`, fail to compile; neither runs silently.

**6. Paths written outside the worktree**
- `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/c-conv/adv1/` (467 MB): `probe.sh`, `twice.sh`, `goenv`, `p/*` (probes), `twice/`, `mut/*` (25 mutant source copies and binaries, `build.sh`, `run.sh`, `kill.sh`, `all.sh`, `mutate.py`, `all.log`), `red-alone*.log`, `alone3.log`, `alone4.log`, `suite.log`, `suite2.log`.
- Inside the tree, untracked under `target/`: `target/c-read-adversary2/` (1.1 MB).

`git status --short`:
```
?? adapters/c-read/tests/adversary2.rs
```

**7. Findings block**
```findings
- file: adapters/c-read/src/rtbody.rs
  line: 596
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "builder chains, cs_string_cat/eq and cs_div/rem with two calling arguments are read in left-to-right order while C (gcc) evaluates them right to left"
- file: adapters/c-read/src/body.rs
  line: 1218
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "a plain C call with two calling arguments is read as a cir/1 call whose order differs from C's (21002 vs 1012)"
- file: adapters/c-read/src/rtbody.rs
  line: 1074
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a slice loop with an int8_t counter is read as range though C panics at index -128"
- file: adapters/c-read/src/rtbody.rs
  line: 858
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "temporaries assigned to i then a[i] are read as i, a[i] = …, which indexes with the old i where C uses the new one"
- file: adapters/c-read/src/rtbody.rs
  line: 337
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "an assignment or ++ through cs_map_get is read as m[k] = / m[k]++, which inserts the key where C writes outside the map"
- file: adapters/c-read/src/body.rs
  line: 1207
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "calls typed by codespec: func lines do not check argument types, so a []uint64 passed to a []int parameter is written ill-typed without a refusal"
- file: adapters/c-read/src/rtbody.rs
  line: 355
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a receive from a channel the line makes send-only is read as <-ch without a refusal"
- file: adapters/c-read/src/rtbody.rs
  line: 1166
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a map range whose body inserts through an alias of the map is read as range (C 6, Go 5), bypassing the documented put check"
- file: adapters/c-read/src/rtbody.rs
  line: 1086
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "an insertion-ordered C map loop whose body depends on the order is read as an unordered range (C always 0, Go 0,0,1,0,4)"
- file: adapters/c-run/src/lib.rs
  line: 499
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the runner compiles runtime-form sources as one translation unit while the reader reads each file alone, so a macro of one file changes another only in the runner"
- file: adapters/c-read/src/runtime.rs
  line: 100
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "int64_t is always read as int, so the C writer's own int64 and uint signatures are refused"
- file: adapters/c-read/src/rtbody.rs
  line: 1159
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "rt_range refuses every index loop over xs.len whose body does not start with the element read, instead of leaving it to the exact plain for reading"
- file: adapters/c-read/tests/runtime.rs
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "18 non-equivalent side-condition mutants of rtbody.rs/runtime.rs survive the suite, and four recognised forms have no near-miss test; adversary2.rs holds a kill case for each"
```
