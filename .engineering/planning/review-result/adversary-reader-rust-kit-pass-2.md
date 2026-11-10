---
format: aep.planning-md/3
id: review-result:adversary-reader-rust-kit-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the Rust reader's kit unit
relations:
- reviews: story:reader-rust-kit
revision: 1
---
unit: story:reader-rust-kit, branch `impl/reader-rust-kit` at `acb92f7` (rd-rust-kit tree, plus one untracked test file)
verdict: NEEDS-CHANGE
cases: executed 95→114, red 13
origin: introduced 4 / pre-existing 6 / undecided 0
wrote-outside-worktree: 1 path (`adv2/` scratch, see part 6)
needs-coordinator: none

The 95 comes from the implementing state's own run in `all3-test.log` (3 lib + 16 adversary1 + 2 cases + 74 reader).

**1. `git --no-pager diff --stat`**
It prints nothing. `git status --short` shows only `?? adapters/rust-read/tests/adversary2.rs`, which is a test file. I changed no implementation file, no existing test and nothing under `.engineering/`. I took my own lease (`adv2-rd-rust-kit-2db1a4b7`) and released it at the end.

**2. Cases added** in `adapters/rust-read/tests/adversary2.rs`, with sources inline. The harness is pass 1's: read, then `codespec pass`, then `codespec eval`, compared with what `codespec-run-rust` prints. A refusal at a named line also passes.

Red now. These are the outputs from the first run of the file alone (`red-first.log` and `red2.log`):
- `a_raw_keyword_local_and_a_raw_local_spelled_as_its_unraw_name_stay_two_bindings`: `left: "ok 6006\n" right: "ok 101006\n"`
- `a_raw_keyword_local_and_a_raw_local_spelled_as_its_numbered_alternative_stay_two`: `left: "ok 2006006\n" right: "ok 2101006\n"`
- `a_raw_keyword_function_and_a_raw_function_spelled_as_its_unraw_name_stay_two`: `left: "ok 21021\n" right: "ok 11021\n"`
- `a_private_raw_keyword_type_reads_inside_format`: `foreign: macro arguments: unexpected token, expected ';'`
- `a_method_call_on_a_reference_to_a_reference_uses_the_impl_for_the_reference`: `left: "ok 3\n" right: "ok 300\n"`
- `a_method_call_through_a_mutable_reference_uses_the_impl_for_it`: `left: "ok 3\n" right: "ok 300\n"`
- `a_const_whose_value_is_refused_leaves_no_constant_undeclared`: `unsupported: no constant limit`
- `a_compound_assignment_evaluates_its_value_before_its_place`: `left: "ok 1\n" right: "ok 101\n"`
- `a_compound_assignment_to_a_static_evaluates_its_value_first`: `left: "ok 1\n" right: "ok 101\n"`
- `an_indexed_assignment_evaluates_its_value_before_its_index`: `left: "ok 1205\n" right: "ok 2105\n"`
- `a_display_refused_for_a_string_method_is_not_used_by_format`: `left: "ok \"plain plain\"\n" right: "ok \"L3 plain\"\n"`
- `two_functions_whose_go_spellings_are_one_stay_two_functions`: `left: "ok 5005\n" right: "ok 4005\n"`
- `a_loop_over_a_btreemap_visits_its_keys_in_order`: `left: "ok \"baced\"\n" right: "ok \"abcde\"\n"`

Green now. Each one is red against the mutant it names (`adv2/mut/kill.log`):

| Test | Mutant | Red output on the mutant |
|---|---|---|
| `kill_a_raw_keyword_function_beside_a_function_of_its_go_spelling_stays_apart` | N02 | 5005 vs 4005 |
| `kill_a_raw_keyword_local_skips_every_taken_alternative` | N04 | 4006006 vs 4005006 |
| `kill_a_type_whose_clashing_impls_are_refused_does_not_convert_to_their_traits` | N14 | `no function Parcel.Weight` |
| `kill_both_clashing_impls_are_refused_at_their_lines_and_read_no_method` | N16 | refusals with an empty reason at 17 and 23 |
| `kill_a_refused_immutable_static_is_named_at_its_use` | N24 | fails |
| `kill_a_reference_to_a_reference_as_a_trait_object_is_refused_at_the_call` | N25 | 3 vs 300 |

**3. Suite run**, after the cases existed: `cargo test -p codespec-read-rust --locked --no-fail-fast`
- lib: 3 passed
- adversary1: 16 passed
- adversary2: 6 passed, 13 failed
- cases: 2 passed
- reader: 74 passed
- exit 101

**4. Findings** (they cover `acb92f7`; base means 767110c, which I built from `git archive` in scratch)

| # | sev | verdict | origin | file:line | finding |
|---|---|---|---|---|---|
| G1 | blocker | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:293 | **F1 residue.** `r#x`, where `x` is not a keyword, is the plain name `x`, but `plain_idents` skips every `r#` token. So `r#type_` and `r#type_1` are not counted as taken, and `r#type` merges with them (6006, 2006006). The function form (21021) also reproduces at base. The local form reads right at base (101006). **Fix:** add the bare word of every non-keyword raw identifier to `plain` before choosing spellings. |
| G2 | warning | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:278 | **F7 residue.** A private `struct r#match` is named `unexported("match")` = `match`, which is the keyword, so a `format!` that uses it is refused. Base read this right ("3 4"). The fix commit's claim "inside macro arguments the name is never a keyword" does not hold for types. |
| G3 | warning | NEEDS-CHANGE | introduced | adapters/rust-read/src/body.rs:3434 | **F5 residue.** `no_ref_impl` guards generic calls and `to_iface`, but not direct method calls. `rr.score()` with `rr: &&Count`, or `p.score()` with `p = &mut c`, calls the impl for `&Count` / `&mut Count` in Rust. The read calls `Count`'s (3 vs 300). The only refusal is at the impl's own line. Base refused trait impls altogether. |
| G4 | warning | NEEDS-CHANGE | pre-existing | adapters/rust-read/src/lib.rs:1830 | **F6 sibling, for consts.** A `const` whose value is refused stays in `consts`, so each use reads as an undeclared constant (`no constant limit`) with no refusal at the use. Base does the same. |
| G5 | blocker | NEEDS-CHANGE | pre-existing | adapters/rust-read/src/body.rs:2492 | For `place op= value` on integers, Rust evaluates the value first. The read gives `cir/1`, which loads the place first (1 vs 101). This reproduces at base through `Rc<RefCell<T>>`. The unit's `static mut` adds a second, natural path to it (`TOTAL += bump()`). |
| G6 | blocker | NEEDS-CHANGE | pre-existing | adapters/rust-read/src/body.rs:2458 | For `v[i()] = val()`, Rust evaluates `val()` before the index. The read evaluates the index first (1205 vs 2105). The `Vec` form reproduces at base; the unit's arrays and statics reach it too. |
| G7 | warning | CONFIRMED | pre-existing | adapters/rust-read/src/body.rs:4328 | `impl Display` is refused at its line when the type has a method `string` (Go `String`). But `format!("{}", l)` still reads as `%s` and calls that method ("plain" vs "L3"). Base does the same. |
| G8 | blocker | NEEDS-CHANGE | pre-existing | adapters/rust-read/src/lib.rs:1867 | `fn value_2` and `fn value2` are both named Go `value2`. One is dropped and both calls go to the other (5005 vs 4005). Locals are renamed apart, but functions are not. Base does the same. |
| G9 | blocker | NEEDS-CHANGE | pre-existing | adapters/rust-read/src/body.rs:1723 | A loop over a `BTreeMap` reads as a `for_range` over a `cir/1` map, whose order is unspecified (`unordered_iteration`). Rust visits the keys sorted ("abcde"); the evaluator gives "baced". Base does the same. |
| G10 | note | CONFIRMED | introduced | adapters/rust-read/tests/reader.rs | Of 28 mutants of `acb92f7`'s checks, reader.rs + adversary1.rs leave 13 alive. The six reachable ones (N02, N04, N14, N16, N24, N25) are now killed by adversary2. Seven behave the same on valid Rust: N01, N03, N12, N15, N20, N23, N28. Two of the checks behind them never fire on valid Rust: the `!matches!(.., syn::Type::Path(_))` test at lib.rs:869 (`last_seg` already requires a path), and `self.find(i).is_none()` in `statics_only` (Rust error E0530 forbids a binding that shadows a static). |

What reaches each finding:
- G1, G2: valid raw identifiers. G1 is contrived: it needs both `r#type` and `r#type_`.
- G3: an impl for `&T` or `&mut T` next to one for `T`.
- G4: a `const` with a non-literal value. That is common.
- G5, G6: a call with a side effect inside an assignment. Common with statics and `RefCell`.
- G7: a `string` method next to `Display`.
- G8: two function names that differ by `_` before a digit.
- G9: any `BTreeMap` loop whose output depends on order. That is common.
- Severity: G1 and G5–G9 are blockers because each is a silent wrong read, which your brief rates blocker. G1's case is contrived (`r#type` next to `r#type_`).

**5. Attacked and held**
- **Unraw:**
  - field shorthand `S { r#type, type_ }`
  - raw and plain methods on two types and on one type
  - `TYPE` next to `r#type`
  - `type_` and `type_1` next to `r#type`
  - a type and a function, or a type and a local, of one raw name
  - a file with a BOM, and a file with a shebang line
- **F3:**
  - generic-bound use
  - `Trait::m(&x)` and `<T as Tr>::m` are refused
  - default methods and supertraits are refused
- **F4:** impls for `Vec`, a tuple, an array, `Option`, `Box`, a slice, `fn`, `dyn`, `&&T`, `&'a mut T`, and a blanket impl are all refused. No reader panic.
- **F6:** a refused static used in a closure, via `crate::`, as an assignment target, or in another static's initialiser is refused at the use.
- **F2:** the tuple-of-`Vec` field, generic `Cell<Vec>`, nested array-of-struct, struct update `..`, and array-of-arrays cases were all refused rather than misread.
- **Generics:**
  - two instantiations of one function
  - recursion
  - a generic struct's methods
  - a type parameter shadowing a struct
  - an integer literal typed only through a generic is refused
- **Integers:**
  - `wrapping_*` at i8, u8, u16 and i64::MIN
  - `%` and `/` signs
  - u64 max
  - a shift by a `u32`
- **Evaluation order and control flow:**
  - a static read before a call
  - same-block and nested shadowing
  - `match` guards
  - `continue` with post-step
  - inner-loop `continue`
- **Strings:** comparison order is right. Widths without alignment are refused.

**6. Paths written outside the worktree**
- `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/rust-kit/adv2/` (5 MB after cleanup): probes, logs and the mutation scripts.
- I deleted its build outputs: `mut-target`, `base-target`, `base`, `build`, `mut/ws`.
- Inside the tree, gitignored: `target/adversary2/` (348 KB), written by the new test file.

**7.**
```findings
- file: adapters/rust-read/src/lib.rs
  line: 293
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Unraw skips non-keyword raw identifiers when collecting taken names, so r#type merges with r#type_ or r#type_1 and a local or function is lost without a refusal."
- file: adapters/rust-read/src/lib.rs
  line: 278
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a private type declared as r#match is named match, a keyword, so a format! using it is refused where the base read it."
- file: adapters/rust-read/src/body.rs
  line: 3434
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a direct method call on &&T or &mut T calls T's impl although the crate implements the trait for &T or &mut T, refused only at that impl's line."
- file: adapters/rust-read/src/lib.rs
  line: 1830
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "a const whose value is refused stays registered, so each use reads as an undeclared constant and the read does not run."
- file: adapters/rust-read/src/body.rs
  line: 2492
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "a compound assignment is read with Go's order, place before value, where Rust evaluates the value first, so a value that changes the place (RefCell field, static mut) reads 1 for Rust's 101."
- file: adapters/rust-read/src/body.rs
  line: 2458
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "an indexed assignment evaluates the index before the value, where Rust evaluates the value first, so side effects run in the other order."
- file: adapters/rust-read/src/body.rs
  line: 4328
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "a Display impl refused for a String-spelled method still lets format! read the value with %s, which calls that method instead of Display."
- file: adapters/rust-read/src/lib.rs
  line: 1867
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "two functions whose Go spellings are one (value_2, value2) become one function without a refusal."
- file: adapters/rust-read/src/body.rs
  line: 1723
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "a loop over a BTreeMap reads as an unordered cir/1 map loop, losing Rust's sorted key order."
- file: adapters/rust-read/tests/reader.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "13 of 28 mutants of acb92f7's checks survived the suite; the six reachable ones are now killed by adversary2 and seven behave the same on valid Rust, two through checks that never fire on valid Rust."
```
