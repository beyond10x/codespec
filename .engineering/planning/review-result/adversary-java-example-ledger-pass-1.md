---
format: aep.planning-md/3
id: review-result:adversary-java-example-ledger-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:java-example-ledger'
relations:
- reviews: story:java-example-ledger
revision: 1
---
unit: story:java-example-ledger, uncommitted working tree on impl/java-example-ledger (base 034277e)
verdict: NEEDS-CHANGE
cases: executed 79→81, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (`<session scratch>/wave-sbw/java-example-ledger/adversary/`)
needs-coordinator: none

**1. Diff stat.** The implementor changed `write.go`, `write_test.go`, `write.rs`, `crates/harness/src/lib.rs` and `crates/harness/tests/harness.rs` (451 insertions, 16 deletions). The adversary added two test files: `adapters/go/cmd/codespec-write-go/adversary_ledger_test.go` and `adapters/rust/tests/adversary_ledger.rs`.

**2. Cases added.** Both are red.
- `TestAdversarySortedKeysBesideALocalNamedLikeAPackageItUses`: a parameter named `sort`, `utf16` or `string` shadows the closure, and `go vet` fails with `sort.Slice undefined`.
- `adversary_a_conversion_on_the_left_of_less_than_is_written_or_refused`: for `int32(len(xs)) < n`, the Rust writer exits 1 with `internal: expected ','`.

**3. Suite.**

| run | result |
|---|---|
| Go | FAIL (the Go case) |
| cargo | 54 ok, 1 failed |
| `examples-check` | exit 0 |
| `examples-ledger` | exit 0, 3 implementations byte-equal |

**4. Findings**
1. `sorted_keys.go:24`, blocker. The `sorted_keys` closure uses `sort`, `utf16` and `string` without qualification. Ordinary Java code reaches this: a `keys(int sort)` reads with 0 refusals, and the Go written from it does not compile.
2. `write.rs:2730`, blocker. A conversion written with `as` keeps the precedence of a cast, so `… as i32 < n` gets no parentheses. The writer then exits 1 instead of writing the code or refusing it. Java's `items.size() < n` reaches it.

**5. Attacked and could not break**
- `sorted_keys` order: supplementary characters, empty strings, `i64` extremes.
- Enum keys: refused by the reader.
- Narrowing with `as`.
- Upper-casing and the `_` suffix chains.
- The `this` value rewrite when `this` escapes.
- Mutants: the harness caught 9 of 10. The tenth is equivalent.
- `int`/`long` edges.
- The other examples are unchanged.

```findings
- file: adapters/go/cmd/codespec-write-go/sorted_keys.go
  line: 24
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the sorted_keys closure names sort, utf16 and string unqualified, so a Java parameter or local of that name (read with 0 refusals) yields Go that does not compile, written with exit 0 and no refusal
- file: adapters/rust/src/write.rs
  line: 2730
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a conv written as `as` keeps CAST precedence, so as the left operand of `<` (Java `items.size() < n`) it prints `… as i32 < n`, which syn rejects, and the writer exits 1 with an internal error instead of writing or refusing
```
