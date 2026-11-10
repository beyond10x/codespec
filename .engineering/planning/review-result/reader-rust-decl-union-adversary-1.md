---
format: aep.planning-md/3
id: review-result:reader-rust-decl-union-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the Rust reader's decl.union claim
relations:
- reviews: story:reader-rust-decl-union
revision: 1
---
unit: story:reader-rust-decl-union, tree `readers-next` at 3ff3b9b7 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed n/a→4 (only the new `adversary_union` target ran; no "before" count was reported to me and I did not run the suite), red 4
origin: introduced 2 / pre-existing 3 / undecided 0
wrote-outside-worktree: 1 path (scratch directory, listed in part 6)
needs-coordinator: free disk on `/` fell to 19G after my last build. That is under your 20 GB stop line, so I did not run `--test cases` or the wider suite.

1. Diff. `git --no-pager diff --stat` is empty. `git status --short` shows:
   - `?? adapters/rust-read/tests/adversary_union.rs`: mine, a test file.
   - `?? .engineering/planning/story/reader-check-stops-when-reached.md`: not mine; another session wrote it.

   I changed no implementation file.

2. Cases added, all in `adapters/rust-read/tests/adversary_union.rs`, all red now. The red output below is from running this file alone:
   - **`adversary_a_union_variant_never_declares_one_field_name_twice`**
     - Red: `union "Pair" variant "Both" declares the field Value2 twice (["Value2", "Value2"]) and nothing is refused; refusals: []`
     - Measured: `lib.rs:2451-2455` turns union fields into exported names, and there is no duplicate check. The struct path has one at `lib.rs:2051-2059` ("both X in Go"). `codespec validate` accepts the module.
     - What reaches it: any enum variant with fields like `value_2` and `value2`, read by `codespec-read-rust` on a user crate. No kit case contains this.
   - **`adversary_an_enum_with_variants_named_like_their_structs_compares_values_of_one_type`**
     - Red: `` `"=="` compares operands of two types: … "x":{…"name":"Circle"} … "y":{…"name":"Square"} ``
     - Measured: `lib.rs:1824-1835` absorbs `enum Shape { Circle(Circle), Square(Square) }` as a message enum even when no channel uses it. It is never read as a union, and `a == b` compares a `Circle` with a `Square`.
     - Effects: the module validates; the Go written from it fails to build (`invalid operation: a == b (mismatched types Circle and Square)`); Rust prints `ok false`.
     - What reaches it: the common Rust idiom of an enum wrapping structs of the same name, compared with `==`.
   - **`adversary_unwrap_or_default_of_an_enum_is_its_default_variant_not_its_zero`**
     - Red: `left: "ok false\n"  right: "ok true\n"`
     - Measured: `body.rs:3941` reads `("unwrap_or_default", 0)` as a Go map index, which gives the zero value. Rust gives the `#[default]` variant (`Blue`).
     - `codespec-run-rust` on the same source printed `ok true`.
     - What reaches it: a `HashMap<_, E>` lookup with `unwrap_or_default()` where `#[default]` is not the zero variant.
   - **`adversary_unwrap_or_default_of_a_union_is_refused_or_runs_as_rust`**
     - Red: `codespec eval stops (exit status: 1): codespec eval: stopped at probe.Defaults#1.rhs.0 (lib.rs:15): unsupported: the zero value of Shape is outside the evaluator`
     - Same line as the previous case, on a union with `#[default] Empty`.
     - Rust prints `ok true`. The reader records no refusal, so a claimed union read stops in the evaluator instead.

3. Suite run. Not run. The command was `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 RUSTC_WRAPPER= cargo test -p codespec-read-rust --test adversary_union --locked`. Result: `test result: FAILED. 0 passed; 4 failed`, cargo exit 101 (the background wrapper reported 0). After that build free disk was 19G, so I stopped as you instructed. `--test cases` is still to be run.

4. Findings from command output:
   - **`adapters/rust-read/src/lib.rs:176`, introduced, note.** The edit removed the blank `//!` line after the last table row, so the row ends in a literal `|//!`. `grep -n '|//!$'` finds it at 176 on HEAD and 0 times at the base. It renders as a stray `//!` cell.
   - **`conformance/cases/decl-union/rust/case.rs:1`, introduced, note.** The new `decl.union` claim cannot detect a wrong union. I replaced the read union's variants with a single fieldless `Circle` using `jq`. It still validated, passed `codespec pass`, and `codespec eval` on the case's calls printed `ok 7`, the expected output. Only the declaration's presence is checked, because `cir/1` has no expression that builds or inspects a union. A structural check in `tests/cases.rs` (variant names and field types for the case's `Shape`) would catch a wrong mapping.

5. Attacked and not broken:
   - Building union values (struct literal, tuple-variant call, unit variant of a union, `Shape::default()`), `if let`, `match` with guards, bindings and `@`, `matches!`: each is refused at its line.
   - Generic enum: refused.
   - Raw variant and field names (`r#match`→`match_`, `r#type` beside `type_`→`Type1`/`Type`): distinct names.
   - `#[repr(u8)]` with explicit discriminants on a data enum: the discriminants are dropped, but safe Rust cannot observe them.
   - The `tests/cases.rs` fallback rule: it is stricter than conform's step 4, which falls back on any Go writer refusal. I found no Go writer failure it hides that conform does not.

6. Paths written outside the worktree: `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rn/adv-rust`. It holds probe crates `p1`–`p5`, their reads and calls, the Rust runner builds `p4-run` and `p5-run`, `p3go`, `p3go-build`, `du*.json` and `run1.log`. Inside the worktree the tests write `target/adversary-union/`.

- adversary_union.rs `adversary_a_union_variant_never_declares_one_field_name_twice`: `union "Pair" variant "Both" declares the field Value2 twice (["Value2", "Value2"]) and nothing is refused; refusals: []`
- adversary_union.rs `adversary_an_enum_with_variants_named_like_their_structs_compares_values_of_one_type`: `` `"=="` compares operands of two types: … "name":"Circle" … "name":"Square" ``
- adversary_union.rs `adversary_unwrap_or_default_of_an_enum_is_its_default_variant_not_its_zero` and `adversary_unwrap_or_default_of_a_union_is_refused_or_runs_as_rust`: `left: "ok false\n" right: "ok true\n"` / `stopped at probe.Defaults#1.rhs.0 (lib.rs:15): unsupported: the zero value of Shape is outside the evaluator`
- command output, lib.rs doc: `grep -n '|//!$'` → `176` on HEAD, `0` at origin/main
- command output, decl-union case: union gutted to `[{"name":"Circle","fields":[]}]` → `ok 7`, exit 0

```findings
- file: "adapters/rust-read/src/lib.rs"
  line: 2453
  category: "boundary"
  severity: "warning"
  origin: "pre-existing"
  verdict: "NEEDS-CHANGE"
  message: "A union variant whose fields share one exported name (value_2, value2) reads to a union declaring Value2 twice with no refusal while the struct path refuses it; red test adversary_a_union_variant_never_declares_one_field_name_twice."
- file: "adapters/rust-read/src/lib.rs"
  line: 1834
  category: "property"
  severity: "warning"
  origin: "pre-existing"
  verdict: "CONFIRMED"
  message: "An enum whose variants are named like the structs they wrap is absorbed as a message enum even without a channel, so == of two of its values reads as Circle == Square, which validates and whose Go does not build; red test adversary_an_enum_with_variants_named_like_their_structs_compares_values_of_one_type."
- file: "adapters/rust-read/src/body.rs"
  line: 3941
  category: "property"
  severity: "warning"
  origin: "pre-existing"
  verdict: "CONFIRMED"
  message: "unwrap_or_default on a map lookup reads as the zero value, so an enum's #[default] variant is lost (evaluator ok false, Rust ok true) and a union's zero stops the evaluator unsupported with no refusal; red tests adversary_unwrap_or_default_of_an_enum_is_its_default_variant_not_its_zero and adversary_unwrap_or_default_of_a_union_is_refused_or_runs_as_rust."
- file: "adapters/rust-read/src/lib.rs"
  line: 176
  category: "judgement"
  severity: "note"
  origin: "introduced"
  verdict: "NEEDS-CHANGE"
  message: "The crate-doc edit removed the blank //! line after the last unclaimed-feature table row, which now ends in a literal |//! (grep output, absent at the base)."
- file: "conformance/cases/decl-union/rust/case.rs"
  line: 1
  category: "mutant"
  severity: "note"
  origin: "introduced"
  verdict: "CONFIRMED"
  message: "The decl.union claim holds for any union the reader emits: the read module with its union replaced by one fieldless variant still validates, passes and evaluates to ok 7, so nothing checks the variants or field types (command output)."
```
