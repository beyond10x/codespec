---
format: aep.planning-md/3
id: review-result:adversary-reader-rust-kit-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the Rust reader's kit unit
relations:
- reviews: story:reader-rust-kit
revision: 1
---
The adversary pass made the suite fail: 9 new cases fail on the unit's code, three of them silent wrong reads of `blocker` severity.

```
unit: story:reader-rust-kit, branch impl/reader-rust-kit at 905a0da (rd-rust-kit tree; adds one untracked test file)
verdict: NEEDS-CHANGE
cases: executed 73→89, red 9
origin: introduced 8 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 path (scratch dir adv1/, see part 6)
needs-coordinator: F8 and F9 are in codespec-write-go, outside this unit's scope, so they need routing. I did not take a worktree session lease.
```

The 73 is the implementing run's count (`all2-test.log`).

**1. `git --no-pager diff --stat`**: prints nothing, because the only change is an untracked file. `git status --short` gives `?? adapters/rust-read/tests/adversary1.rs`. It is a test file. No implementation file, existing test, `.engineering` path or git state changed. The test sources are inline, so there is no `tests/adversary/` fixtures directory.

**2. Cases added** in `adapters/rust-read/tests/adversary1.rs`. Each case reads a source, runs `codespec pass` and `codespec eval`, and compares the output to what `codespec-run-rust` printed for the same source. A refusal at a named line also passes.

The first run of this file alone had 7 of the 9 cases failing because my own `calls.json` was not canonical. I fixed that harness bug, then ran the file alone again:
- `raw_local…stay_two_bindings`: `left: "ok 203\n" right: "ok 106\n"`, refusals: []
- `raw_parameter…stay_two_parameters`: `left: "ok 0\n" right: "ok 7\n"`, refusals: []
- `raw_function…stay_two_functions`: `left: "ok 30030\n" right: "ok 30004\n"`, refusals: []
- `clone_of_an_array_of_vectors…`: `left: "ok 99\n" right: "ok 1\n"`, refusals: []
- `two_traits_with_one_method_name…`: `left: "ok 4004\n" right: "ok 14004\n"`, refusals: []
- `crate_trait_implemented_for_a_primitive…`: `codespec eval: …: the module declares no type i64; refusals: []`
- `generic_call_on_a_reference…`: `left: "ok 4\n" right: "ok 400\n"`, the only refusal is at `lib.rs:12` (`impl &Count`)
- `static_whose_initialiser_is_refused…`: `not a cir/1 module: global slots is not declared`
- `raw_name…keyword_reads_inside_format`: refused at `lib.rs:3` with reason "macro arguments: unexpected end of input, expected an expression"

Seven more cases are green on the unit's code. Each one fails against the mutant it was written for (see F10): M03, M06, M08, M09, M11, M17 and M20.

**3. Suite run**, after all cases existed: `cargo test -p codespec-read-rust --locked --no-fail-fast`
- lib unit tests: 3 passed
- adversary1: 7 passed, 9 failed
- cases: 2 passed
- reader: 68 passed
- exit 101

**4. Findings** (this covers 905a0da plus the untracked test file)

| # | sev | verdict | origin | file:line | finding |
|---|---|---|---|---|---|
| F1 | blocker | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:278 | `Unraw` renames `r#type` to `type_`, which is also a legal plain Rust name. Two Rust names become one. As locals, the later binding shadows the earlier (203 vs 106). As parameters, it reads 0 vs 7. As functions, one function is dropped with no refusal (30030 vs 30004). Fix: check the new spelling against every plain identifier in the file and add `_` until it is unique, as is already done for type names. |
| F2 | blocker | NEEDS-CHANGE | introduced | adapters/rust-read/src/body.rs:3378 | `shares_storage` has no arm for `Type::Array`. So `.clone()` of a struct holding `[Vec<i64>; 2]` is read as a Go copy that shares the vectors' storage (99 vs 1). Fix: add `Type::Array { elem, .. } => self.shares_storage(elem, seen)`. |
| F3 | blocker | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:1760 | Signatures are keyed by (type, Rust method name). When two crate traits share a method name on one type, one body is lost (4004 vs 14004). If the two traits differ in visibility, the module holds two `weight` methods and calls a `Weight` method that does not exist. Fix: refuse this case, as is already done when a trait method clashes with an inherent one. |
| F4 | warning | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:789 | `impl Measured for i64` is read without a refusal as methods on an undeclared type `i64`. The evaluator refuses the module, and Go fails with `undefined: i64`. `impl_scope` accepts any non-generic undeclared type, and `lib.rs:1633` registers it. |
| F5 | warning | CONFIRMED | introduced | adapters/rust-read/src/body.rs:3847 | `impl Score for &Count` is refused at its own line. But `total(&c)` drops the `&` and is read as an instantiation with `Count`, using `Count`'s impl (4 vs 400). The call's line has no refusal. Rare: it needs both `T` and `&T` impls with different bodies. |
| F6 | warning | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:1720 | A static is registered whether or not its initialiser reads. When it is refused (`[0; 3]`, at `:2125`), every use still reads as `global`, and the output fails `codespec validate`. The refusal also says "a constant" for a static. |
| F7 | warning | NEEDS-CHANGE | introduced | adapters/rust-read/src/lib.rs:283 | `Unraw::tokens` turns `r#match` inside a macro into the keyword `match`. A readable `format!("{}", r#match)` is then refused with a parse error as its reason. |
| F8 | warning | CONFIRMED | pre-existing | adapters/go/cmd/codespec-write-go/write.go:999 | The Go writer writes float literals as untyped constants, and Go folds them exactly. The read is right (eval agrees with Rust); only the written Go differs: `0.1 + 0.2 == 0.3` gives Rust false, Go true; `1.0 / -0.0 < 0.0` gives Rust true, Go false; f32 `3.0e38 * 10.0 / 10.0 == 3.0e38` gives Rust false, Go true. Now reachable through the claimed `expr.float`. |
| F9 | warning | CONFIRMED | pre-existing | adapters/go/cmd/codespec-write-go/write.go:1005 | The Go writer gives a const or global the same name a local can take, so the local shadows it. The read is right; only the written Go differs: `static mut TOTAL` with `let total` gives Rust 5,5, Go 10,0; `const LIMIT` with `let limit` gives Rust 8, Go 10. Statics now reach it. |
| F10 | note | CONFIRMED | introduced | adapters/rust-read/tests/reader.rs | The unit's suite kills only 4 of 24 mutants of its new checks (M05, M13, M16, M22). My kill tests now catch M03, M06, M08, M09, M11, M17 and M20. Equivalent on valid Rust: M01, M02, M04, M07, M12, M14, M15, M18, M21. Reachable and still not killed: M10 (needs `struct r#match` beside `Match` and `Match_`), M19 (`impl<T: Tr> S<T>` would produce Go that does not compile), M23 (the reader panics instead of refusing), M24 (comparing `fn` values would produce Go that does not compile). |

What reaches each one: F1, F2 and F3 are ordinary valid Rust. F4 needs a crate trait implemented for a primitive. F6 needs a static with a non-literal initialiser. F7 needs a raw identifier inside a macro. F8 needs float literal arithmetic, F9 a local named like a const or static. F5 is the rare `T`/`&T` pair.

**5. Attacked and held**
- Closures: shared and mutated captures, nested closures, a name bound again after the closure is made, and `{name}` captures inside a closure all matched Rust. `move` closures are refused.
- Generics: swapped type parameters, a struct literal with fields out of order, `Pair::make` inferred from arguments, a `T: Trait` bound, a type parameter named like a struct, and an impl naming the struct's parameters out of order (refused).
- Derived `Default`: nested derived structs match. A hand-written nested `Default`, a pointer two levels down, and a clone of a defaulted struct holding a `Vec` are all refused.
- Floats: formatting and `to_string` are refused. A literal with no type context is refused. Float `%` is refused.
- `Rc<dyn Any>` and channels: an untyped literal in `Rc::new` is refused. Only `try_send` and `try_recv` followed by `expect` or `unwrap` are read.
- `sorted_keys`: string keys sorted by byte and a reversed comparator do not read as `sorted_keys`.
- A trait method named like a field: the Go writer renames the field.
- Determinism: every probe read twice was byte-equal.

**6. Paths written outside the worktree**
- `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/rust-kit/adv1/` (134 MB): probes, logs and the mutation workspace.

Inside the worktree, all gitignored under `target/`, I created these new paths, about 1 GB in total. Most of it is `target/adv1-mut-cases` (783 MB), which can be deleted:
- `target/adv1-go`
- `target/adv1-mut-cases`
- `target/adversary1`
- `target/adversary1-mut`
- `target/debug/adv1-mut-read-rust`
- `target/debug/deps/*adv1_mut_rr*`

Free disk was 43 GB at the end.

**7.**
```findings
- file: adapters/rust-read/src/lib.rs
  line: 278
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Unraw renames r#type to type_, merging it with a plain type_ local, parameter or function, so reads bind the wrong name or drop a function without a refusal."
- file: adapters/rust-read/src/body.rs
  line: 3378
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "shares_storage has no Array arm, so cloning a struct holding an array of Vecs reads as a Go copy that shares their storage."
- file: adapters/rust-read/src/lib.rs
  line: 1760
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "two crate traits with one method name on one type collapse into one signature, losing one body without a refusal."
- file: adapters/rust-read/src/lib.rs
  line: 789
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a crate trait implemented for a primitive reads without refusal as methods of an undeclared type the evaluator and Go reject."
- file: adapters/rust-read/src/body.rs
  line: 3847
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a generic call on &T is read with T's impl when an impl for &T exists and is refused only at its own line."
- file: adapters/rust-read/src/lib.rs
  line: 1720
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a static whose initialiser is refused still reads as a global at every use, so the module fails codespec validate."
- file: adapters/rust-read/src/lib.rs
  line: 283
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a raw identifier whose bare word is a Rust keyword becomes that keyword inside macro tokens, refusing a readable format! with a parse error as its reason."
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 999
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "float literals written as untyped Go constants fold exactly, so a correct read of 0.1 + 0.2 or -0.0 runs differently from Rust and the evaluator."
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 1005
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "a const or global is written with the same identifier a local can take, so the local shadows it in the written Go."
- file: adapters/rust-read/tests/reader.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the unit's suite kills 4 of 24 mutants of its new checks; M10, M19, M23 and M24 are reachable and still not killed."
```
