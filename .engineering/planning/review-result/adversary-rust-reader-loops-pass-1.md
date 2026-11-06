---
format: aep.planning-md/3
id: review-result:adversary-rust-reader-loops-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:rust-reader-loops'
relations:
- reviews: story:rust-reader-loops
revision: 2
---
unit: story:rust-reader-loops, uncommitted working tree in codespec-w1-loops (base 657dde7)
verdict: NEEDS-CHANGE
cases: executed 20→25, red 5
origin: introduced 4 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 paths
needs-coordinator: none

**1. Diff stat (worktree against base 657dde7)**
```
 adapters/rust-read/src/body.rs     | 608 ++++++++++++++++++++++++++++++++++++-
 adapters/rust-read/tests/reader.rs | 431 +++++++++++++++++++++++++-
```
The `body.rs` lines are the implementor's. Its stat (608 lines; numstat 598/10) is the same as before my work. I ran `cargo fmt -p codespec-read-rust`, and it left `body.rs` unchanged. Every line I added is in `tests/reader.rs`.

**2. Cases added** (`adapters/rust-read/tests/reader.rs`, all 5 red now)

I wrote all five before running anything, then ran them alone with `cargo test -p codespec-read-rust --test reader adversary_`. I then edited two of my own cases so their input has no refusal at all, which makes the misreading silent. I re-ran those two alone, and they were red again with `refusals []`.

- **`:558` `adversary_a_value_block_whose_later_target_reads_an_earlier_target_is_not_a_multi_assign`**
  - Input: `{ let __value0 = 1i64; let __value1 = 2i64; i = __value0; s[i as usize] = __value1; }`
  - Red: `read as \`i, s[i] = 1, 2\`, which indexes with the old \`i\`: [Assign { op: "", lhs: [Var i, Index { x: Var s, index: Var i }], rhs: [Lit 1, Lit 2] ...`
  - Go reads the index before it assigns, so the meaning differs. The writer refuses exactly this case (`assign_many`, adapters/rust/src/write.rs).
- **`:584` `adversary_a_last_statement_reading_a_body_local_that_shadows_an_outer_name_is_not_the_post`**
  - Input: an outer `let step = 1`, then a body with `let step = 2; ...; i = i.wrapping_add(step);`
  - Red: `post Assign { op: "+=", lhs: [Var i], rhs: [Var { name: "step" }] } in For { ..., body: [Let { names: ["step"] ...`
  - The post sits outside the body's scope, so the document steps by 1 where Rust steps by 2.
- **`:610` `adversary_a_slice_loop_block_whose_body_reassigns_the_slice_is_not_a_range`**
  - Input: the writer's `__len` slice block, with a body that does `s = Vec::new();`
  - Red: `read as a range over \`s\`, which Go evaluates once: ForRange { key: "i", val: "v", over: Var s, body: [..., Assign { lhs: [Var s], rhs: [Make ...] }] }; refusals []`
  - In Rust the second iteration panics. In Go the loop completes over the old slice.
- **`:637` `adversary_an_empty_collection_is_not_typed_from_a_binding_the_assignment_does_not_name`**
  - Input: `fn f(out: Vec&lt;i64&gt;) -&gt; Vec&lt;String&gt; { let v = Vec::new(); let mut out: Vec&lt;String&gt; = Vec::new(); out = v; out }`
  - Red: `` `v` typed from the parameter `out`, not the `out` it is assigned to: Slice { elem: Int { width: W64, signed: true } }; refusals [] ``
  - The document assigns a `[]int64` to a `[]string`.
- **`:663` `adversary_a_length_cast_to_a_narrower_type_is_a_conversion_or_refused`**
  - Input: `v.len() as u8`
  - Red: `read as \`len(v)\` typed uint8, without a conversion: [Return { results: [Intrinsic { name: "len", ..., t: Int { width: W8, signed: false } }] }]`
  - Rust truncates here, and the document has no `Conv` (cir/1 has `Expr::Conv`).

**3. Suite run** (after the cases existed)

`CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo test -p codespec-read-rust`
```
unittests src/lib.rs: test result: ok. 2 passed; 0 failed
unittests src/main.rs: test result: ok. 0 passed
tests/reader.rs: test result: FAILED. 18 passed; 5 failed; 0 ignored
error: test failed, to rerun pass `-p codespec-read-rust --test reader`
EXIT=101
```
The before count is 20: this same run with my five `adversary_*` cases left out (2 + 18).

**4. Findings** (they cover the uncommitted tree on base 657dde7)

What reaches every finding: any hand-written Rust crate passed to `codespec read --lang rust`. The reader's job is to read such a crate strictly. The writer's own output for `series`, `orders` and `inventory` does not produce these forms; the writer refuses case 1 outright.

All five are silent: the reader records no refusal for them.

1. **`body.rs:107`/`:982` (`value_block`, `assign_values`), blocker, NEEDS-CHANGE, introduced.** A later target whose index or field operand reads a variable an earlier target assigns becomes a Go multi-assign with a different meaning.
   - Fix: refuse the same condition `assign_many` refuses.
   - Related risk, which I reasoned about but did not test: a call inside a target's index runs after the values in Rust and before them in Go.
2. **`body.rs:1207` (`loop_stmt` post), blocker, NEEDS-CHANGE, introduced.** The last statement becomes the post even when it reads a name the body binds. The post then resolves to an outer binding of that name, or the whole loop becomes foreign if there is none.
   - Fix: take the post only when it reads no name the body's top-level scope binds. The same applies to the text-matched `{ post; continue; }`.
3. **`body.rs:1077` (`range_block`), blocker, NEEDS-CHANGE, introduced.** The check "a variable or a field" is not enough: Go evaluates the operand once, while the Rust form re-reads it on every iteration.
   - Fix: refuse when the body assigns the variable at the root of `over`.
   - I did not test it, but the writer's `range` likely has the same problem in the other direction.
4. **`body.rs:449` (`empty_hint`), warning, NEEDS-CHANGE, introduced.** The assigned place is resolved in the scope at the `let`, not at the assignment, so a target bound in between gets the outer binding's type.
   - Fix: stop the scan at any `let` that rebinds the assignment target's root name.
5. **`body.rs:1685` (len cast), warning, NEEDS-CHANGE, undecided.** `len(x) as &lt;int&gt;` takes the cast's type with no conversion.
   - Fix: emit `Conv` around `len`, or refuse any width other than i64.
   - Origin: at base, the cast was dropped completely (`git show 657dde7:…body.rs:1116`). I did not run base, so I cannot say whether it was misread there in the same way.

**5. Attacked and could not break**
- A bare or labeled `continue` that skips the post: no post is taken, or it is refused.
- Nested loops and closures keeping their own `continue`s.
- A `{ other; continue; }` that does not match the post: refused.
- `__tag` read once, with a non-tag or `&amp;&amp;` condition refused.
- An odd or misnamed `__value` block: refused.
- `break` with a label or a value: refused.
- `break` inside a match: this agrees with the model ("Leaves the innermost enclosing `for`").
- Not checked: whether `orders` and `inventory` still match their checked-in documents. That part of the acceptance is `task examples-check`, which I did not run.

**6. Paths written outside the worktree**
- <session scratch>/wave-sbw/rust-reader-loops/adversary/
- …/adversary/adversary-only.log
- …/adversary/suite.log

**7. Findings block**
```findings
- file: adapters/rust-read/src/body.rs
  line: 107
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a __value block whose later target indexes with a variable an earlier target assigns is read as a Go multi-assign that indexes with the old value, a case the writer refuses
- file: adapters/rust-read/src/body.rs
  line: 1207
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the loop's last statement becomes the post even when it reads a name the body shadows, so the post resolves to the outer binding and the loop steps differently
- file: adapters/rust-read/src/body.rs
  line: 1077
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the slice-loop block is read as a range even when the body reassigns the slice, which Go evaluates once and Rust re-reads each iteration
- file: adapters/rust-read/src/body.rs
  line: 449
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the empty-collection hint resolves the assigned place at the let rather than at the assignment, so it can take an outer binding's type and give the document a mismatched assignment with no refusal
- file: adapters/rust-read/src/body.rs
  line: 1685
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: len(x) as a narrower integer is read as len typed with the cast's type and no Conv, dropping Rust's truncation
```
