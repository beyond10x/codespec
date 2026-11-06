---
format: aep.planning-md/3
id: review-result:adversary-feature-vocabulary-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:feature-vocabulary'
relations:
- reviews: story:feature-vocabulary
revision: 1
---
unit: story:feature-vocabulary, the uncommitted working tree on base 90bb66f in codespec-w1-feature-vocabulary (after correction round 1)
verdict: INFEASIBLE
cases: executed 26→32, red 6
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (`<session scratch>/wave-sbw/feature-vocabulary/adversary2/suite.log`)
needs-coordinator: yes. Decide whether `features_from_json` enforces the module doc's format rules or the doc stops calling the reader strict. The Taskfile patch from pass 1 is still not applied.

**1. Diff stat**

The implementor changed 6 tracked files (34 insertions, 7 deletions): `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/model/src/lib.rs`, `schemas/cir-1.schema.json`, `website/docs/reference/cir-1.md` and `website/docs/reference/cli.md`. The adversary added one untracked test file, `crates/model/tests/adversary2_features.rs`, and changed no implementation file.

**2. Cases added** (`crates/model/tests/adversary2_features.rs`). Each case breaks one rule of the format table in the `features.rs` module doc. All six were red when the file was run alone:

- `adversary2_a_duplicate_id_is_refused`: a duplicate id is accepted.
- `adversary2_unsorted_ids_are_refused`: features in reversed order are accepted.
- `adversary2_an_op_on_a_non_operator_feature_is_refused`: `stmt.for` carrying `"op":"+"` is accepted.
- `adversary2_an_operator_feature_without_its_op_is_refused`: `op.add` without `op` is accepted.
- `adversary2_another_cir_format_is_refused`: `cir/2` is accepted.
- `adversary2_an_unknown_family_is_refused`: family `gadget` is accepted.

Output: `test result: FAILED. 0 passed; 6 failed`

**3. Suite run.** `cargo test --no-fail-fast -p b10x-codespec-model -p b10x-codespec-cli` exited 101.

| test binary | result |
|---|---|
| cli `features` | 2 ok |
| cli `harness` | 2 ok |
| model `adversary2_features` | 0 ok, 6 failed |
| model `adversary_features` | 3 ok |
| model `features` | 12 ok |
| model `model` | 7 ok |

`generate --check`, clippy, fmt, and `codespec schema` and `codespec features` compared with the checked-in files all exited 0.

**4. Findings**

G1, `crates/model/src/features.rs:270` (`features_from_json`): INFEASIBLE, introduced, warning.
- The reader checks only `format` and unknown fields, although the module doc defines six more format rules and calls the reader strict.
- Nothing reaches it yet. The first planned callers are the conformance cases and the adapter support declarations.
- Suggested fix: after deserializing, check sort order and uniqueness, `op.is_some() == (family == "operator")`, `cir == cir/1`, and family membership, each with its own `FeaturesReadError` variant.

Pass 1's F1, F2 and F3 are fixed. The let-chain in `adversary_features.rs:32-36` behaves as the nested `if` it replaced.

**5. Attacked and could not break**
- Drift-test parsing: wrapped lines, `&^`, `<<=`, `&^=` and `<-`.
- The doc comments against both readers: Binop 19 ⊇ 18, Unop ⊇ the Rust set, Assign 11 ⊇ 10.
- A missing required field is refused.
- Determinism.

**6. Paths written outside the worktree:** `<session scratch>/wave-sbw/feature-vocabulary/adversary2/suite.log`

```findings
- file: crates/model/src/features.rs
  line: 270
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: features_from_json accepts duplicate ids, unsorted ids, op on non-operator ids, operator ids without op, cir other than cir/1 and unknown families, though the module doc defines all six as format rules and calls the reader strict; no caller reads a foreign document yet
```
