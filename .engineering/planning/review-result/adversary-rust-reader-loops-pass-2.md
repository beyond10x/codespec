---
format: aep.planning-md/3
id: review-result:adversary-rust-reader-loops-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:rust-reader-loops'
relations:
- reviews: story:rust-reader-loops
revision: 1
---
unit: story:rust-reader-loops, the uncommitted working tree in codespec-w1-loops (base 657dde7), after correction round 1
verdict: NEEDS-CHANGE
cases: executed 25→30, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths under `<session scratch>/wave-sbw/rust-reader-loops/adversary2/`
needs-coordinator: none

**1. Diff stat (worktree against 657dde7)**

`adapters/rust-read/src/body.rs` gained 919 lines; all of them are the implementor's. `adapters/rust-read/tests/reader.rs` gained 598 lines. The adversary added only to the test file, from line 675: the helper `adversary2_body` and five `adversary2_*` cases.

**2. Cases added.** All five are red, and every failure reports `refusals []`.

- `:688` `adversary2_a_value_block_whose_later_target_reads_a_field_an_earlier_target_assigns_is_not_a_multi_assign`
  - Input: `o.i = __value0; s[o.i as usize] = __value1;`
  - Read as `o.i, s[o.i] = 1, 2`, which indexes with the old `o.i`.
- `:718` `adversary2_a_value_block_whose_later_target_reads_an_element_an_earlier_target_assigns_is_not_a_multi_assign`
  - Input: `s[0] = __value0; t[s[0]] = __value1;`
  - Read as a multi-assign that indexes `t` with the old `s[0]`.
- `:746` `adversary2_a_slice_loop_block_whose_body_changes_the_slice_inside_println_is_not_a_range`
  - The body calls `println!("{}", b.reset())`, and `reset(&mut self)` empties `b.items`.
  - The loop is still read as a range over `b.items`.
- `:782` `adversary2_a_slice_loop_block_whose_body_calls_a_crate_method_named_get_is_not_a_range`
  - A crate method `get(&mut self)` empties the slice.
  - The loop is still read as a range.
- `:818` `adversary2_a_value_block_whose_value_reads_its_own_temporary_is_not_a_multi_assign`
  - The block's value reads `__value0`.
  - It resolves to an outer `__value0`.

**3. Suite run.** `cargo test -p codespec-read-rust` exited 101: reader `23 passed; 5 failed`, lib unit tests 2 passed. There are no regressions:
- `codespec read --lang rust examples/series/out/rust` reports 0 refusals.
- `task examples-check` exits 0.
- `task examples-inventory` exits 0, with output byte-equal across 200 seeds x 300 commands for 3 implementations.

**4. Findings.** All are introduced: at base, every block statement and every `while`/`loop` was refused. They are reached by hand-written Rust crates. The writer's output does not reach them.

1. **Multi-value targets.** `body.rs:1278` (`assign_values`), blocker, NEEDS-CHANGE.
   - The check only considers targets that are plain variables. A field or element target that is assigned first and then read by a later target's operand is silently misread.
2. **Macro arguments.** `body.rs:151` (`walk_expr`/`changes`), blocker, NEEDS-CHANGE.
   - The walker does not enter the arguments of `println!`/`format!`, although the reader reads them as expressions.
   - So a `&mut self` method called inside one goes unseen.
3. **Method names.** `body.rs:342` (`changes`, the `READS` exemption), blocker, NEEDS-CHANGE.
   - A method is exempted by its name alone.
   - So a crate method named `get`, `len`, `clone`, `to_string` or `wrapping_*` that takes `&mut self` passes the check.
4. **Writer temporaries.** `body.rs:1292` (`assign_values` values) and `:83`, warning, INFEASIBLE.
   - The writer's temporaries (`__value*`, `__len`, `__i`, `__tag`) are not bound in the document.
   - User code that reads one resolves to an outer binding of that name.

**5. Attacked and could not break**
- Post shadowing through every pattern form.
- A `continue` buried in an expression.
- Mutation through a method chain (`iter_mut`, `mem::take`, `(&mut b).x`).
- A `&mut T` free-function parameter.
- Destructuring assignment.
- Tuple-pattern rebinding in the empty-collection hint.
- Chained and parenthesised `len` casts.
- `wrapping_neg`.

```findings
- file: adapters/rust-read/src/body.rs
  line: 1278
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the multi-value check considers only plain-variable targets, so a field or element target assigned first and read by a later target's index becomes a Go multi-assign that indexes with the old value
- file: adapters/rust-read/src/body.rs
  line: 151
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: changes() does not enter println!/format! arguments, which the reader reads, so a &mut self method called inside one empties the slice and the block is still read as a range
- file: adapters/rust-read/src/body.rs
  line: 342
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: changes() exempts a method by name alone, so a crate method named get (or len, clone, to_string, wrapping_*) taking &mut self passes and the block is read as a range
- file: adapters/rust-read/src/body.rs
  line: 1292
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the writer's temporaries (__value, __len, __i, __tag) are not bound in the document, so user code reading one resolves to an outer binding of the same name with no refusal
```
