---
format: aep.planning-md/3
id: review-result:adversary-feature-vocabulary-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:feature-vocabulary'
relations:
- reviews: story:feature-vocabulary
revision: 1
---
unit: story:feature-vocabulary, the uncommitted working tree on base 90bb66f in codespec-w1-feature-vocabulary
verdict: NEEDS-CHANGE
cases: executed 17→20, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: yes. Someone has to decide whether compound-assignment operators get their own ids (`assign.add` and so on) or are mapped to `op.*` in the descriptions. The Taskfile patch in scratch (`taskfile-features.patch`) is still not applied, so `task schema`, which the drift messages point to, does not regenerate `cir-1.features.json` yet.

**1. Diff stat**

`git --no-pager diff --stat` shows only the implementor's four tracked files: `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/model/src/lib.rs` and `website/docs/reference/cli.md` (4 files, 23 insertions, 3 deletions). My only file is untracked, so `git status` shows it: `?? crates/model/tests/adversary_features.rs`, a test file. I changed no implementation file.

**2. Cases added**, all in `crates/model/tests/adversary_features.rs`. I ran this file alone before running the gate:

- `adversary_every_op_and_intrinsic_in_the_examples_has_a_feature_id`: **red.** It walks every `examples/*/*.cir.json`. Every `binop`, `unop` and `assign` `op` must have an operator feature whose description starts with that op in backticks followed by a colon; every `intrinsic` name must have `intrinsic.&lt;name&gt;`.
- `adversary_every_compound_assignment_the_readers_emit_has_a_feature_id`: **red.** It checks the compound-assignment ops the readers emit (`adapters/rust-read/src/body.rs:792-801`, plus Go's `s.Tok.String()` at `adapters/go/cmd/codespec-read-go/front.go:413-416`).
- `adversary_every_unop_and_intrinsic_the_model_documents_has_a_feature_id`: **green, added as a guard.** It reads the op and intrinsic names in backticks from the schema's `Unop` and `Intrinsic` descriptions and requires an id for each. Today nothing ties the constant lists to those doc comments.

Red output from that first run:
```
---- adversary_every_compound_assignment_the_readers_emit_has_a_feature_id stdout ----
panicked at crates/model/tests/adversary_features.rs:109:5:
compound assignment operators with no feature id: ["+=", "-=", "*=", "/=", "%=", "&amp;=", "|=", "^=", "&lt;&lt;=", "&gt;&gt;=", "&amp;^="]
---- adversary_every_op_and_intrinsic_in_the_examples_has_a_feature_id stdout ----
panicked at crates/model/tests/adversary_features.rs:92:5:
emitted in examples/*/*.cir.json with no feature id: ["assign \"+=\"", "assign \"/=\""]
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

**3. Gate run** (after the cases existed): `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 cargo test --no-fail-fast -p b10x-codespec-model -p b10x-codespec-cli`, exit 101.

The first run without `--no-fail-fast` also exited 101, but it stopped at the first failing binary and so undercounted. Results per test binary:
- cli `tests/features.rs`: 2 passed
- cli `tests/harness.rs`: 2 passed
- model `tests/adversary_features.rs`: 1 passed, 2 FAILED
- model `tests/features.rs`: 6 passed
- model `tests/model.rs`: 7 passed
- unit-test and doc targets: 0 tests

That gives 20 tests in total. The "before" count of 17 is the same run minus my 3 cases.

**4. Findings** (they cover the working tree above)

- **F1, `crates/model/src/features.rs:24` (`BINOPS`/`UNOPS`): NEEDS-CHANGE, introduced.** `Stmt::Assign.op` (`crates/model/src/lib.rs:278-281`) carries compound operators, and no feature id names any of them.
  - **What was measured:** the two red cases above.
  - **What reaches it:** the checked-in examples already contain `+=` 12 times and `/=` twice. The Rust reader emits all ten of its compound ops, and the Go reader emits every Go assignment token, including `&amp;^=`, `&lt;&lt;=` and `&gt;&gt;=`.
  - **Why it matters:** a writer can support `stmt.assign` and `op.div` and still not support `/=`, and the vocabulary cannot say so.
  - **Fix (not applied):** add ids for these operators, or state the mapping to `op.*` in the descriptions and in conformance.md § 2.
- **F2, `crates/model/src/features.rs:53-61` (`UNOPS`, `INTRINSICS`): CONFIRMED, introduced, warning.** These are hand-kept lists, and no drift test compares them with the model's own doc comments (`lib.rs:431-432`, `lib.rs:456`) or with what the readers emit.
  - **What was measured:** nothing in the unit's tests reads those comments. Adding `cap` to the `Intrinsic` doc, or a reader starting to emit `%`, leaves `the_checked_in_feature_vocabulary_is_the_one_the_model_generates` and the count test green. I added the guard case for the documented half; the half the readers emit stays unguarded except through the examples case.
  - **What reaches it:** the next person to add an intrinsic or an op.
  - The story's acceptance does say "a constant list", which is why this is a warning and not a blocker. But design § 2 names "the names `Intrinsic` documents" as the source of these ids.
- **F3, `schemas/cir-1.features.json` / `crates/model/src/features.rs:17`: CONFIRMED, introduced, warning.** `codespec-features/1` is not documented anywhere: no schema, no field definitions in `docs/`, and `Feature` derives only `Serialize`, so there is no strict reader.
  - The operator string an id stands for appears only inside the prose description ("`` `&amp;^`: bit clear ``"). A consumer mapping an `op` it sees in cir/1 to an id, which is what § 3 cases and adapter support declarations need, has to parse prose.
  - **What reaches it:** conformance § 3 and the adapter manifests, both later stories. Nothing reads the format today.
  - **Fix (not applied):** an `op` field for operator features, or a published schema.

**5. Attacked and could not break**
- A new or renamed `k` variant, or a new `Facts` field: it changes `schema_json()`, so the drift test and the count test both fail.
- Ids are sorted, unique and deterministic. The final sort is on `id`, and the CLI-twice test and the in-process test both pass.
- Binops: all 18 the Rust reader emits and all 19 Go binary tokens have ids.
- Unops the readers emit (`!`, `-`, `^`, `*`, `&amp;`, `&lt;-`, `+`): all have ids.
- All five intrinsics the readers emit have ids.
- `sem.switch.break` ("break inside a switch leaves the enclosing loop") matches the model's `Break` doc at `lib.rs:322`, so it is not inverted.
- The unit's tests contain no test that cannot fail. Both drift tests compare against a file read with `unwrap_or_default`, so a missing file fails them.

**6. Paths written outside the worktree:** none. Builds went to the worktree's own `target/`.

```findings
- file: crates/model/src/features.rs
  line: 24
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: compound-assignment ops in Stmt::Assign.op (+= and /= in the examples, ten from the Rust reader, every Go assignment token) have no feature id
- file: crates/model/src/features.rs
  line: 53
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the hand-kept UNOPS and INTRINSICS lists are not checked against the model's Unop/Intrinsic doc comments or the readers, so a newly documented or emitted op or intrinsic leaves every drift test green
- file: crates/model/src/features.rs
  line: 17
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: codespec-features/1 has no schema or field documentation and no strict reader, and an operator id's op string exists only inside its prose description
```
