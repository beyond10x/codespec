---
format: aep.planning-md/3
id: review-result:adversary-java-example-ledger-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:java-example-ledger'
relations:
- reviews: story:java-example-ledger
revision: 1
---
unit: story:java-example-ledger, uncommitted working tree on impl/java-example-ledger (base 034277e), after correction round 1
verdict: NEEDS-CHANGE
cases: executed 97→105, red 8
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (`<session scratch>/wave-sbw/java-example-ledger/adversary2/`)
needs-coordinator: none

**1. Diff stat:** 6 files changed, +518/−19. The adversary added two test files: `adversary2_ledger_test.go` and `adversary2_ledger.rs`.

**2. Cases added (all red)**
- `TestAdversary2SortedKeysBesideAPackageFunctionNamedLikeABuiltin` (subtests `make`, `len`, `append`): the written Go fails `go vet`, and the writer exits 0. The base writer refused `sorted_keys` here.
- `TestAdversary2SprintfBesideALocalNamedFmt`: `fmt.Sprintf undefined`. The base writer writes the same Go.
- `adversary2_a_cast_ending_a_bitwise{,_or,_xor}_left_operand_of_less_than_is_written`: refused with `internal: the written function does not parse`.

**3. Suite**

| run | result |
|---|---|
| Go | 4 adversary2 tests FAIL; 41 others pass |
| cargo | 3 adversary2 tests FAIL; the rest pass |
| `examples-check` | exit 0 |
| `examples-ledger` | exit 0, byte-equal |

**4. Findings**
1. `sorted_keys.go:79`, blocker, introduced. `shadowedIn` checks only the function's own locals. A package-level `make`, `len` or `append` can shadow a builtin the sort code uses.
2. `write.rs:2855`, warning, introduced. Parentheses are added only when the left operand is a cast. In `mask & xs.len() as i64 as i32 < n` the cast ends a bitwise operand instead, so it gets none and the function is refused as `internal`.
3. `write.go:876`, warning, pre-existing. A local named `fmt` shadows the plain `fmt` import.

**5. Attacked and could not break**
- Alias chains across types, consts, funcs and fields.
- A shared alias across functions; output is deterministic.
- A for-init local named `len`.
- A cast on the right-hand side; `<<` refused; receiver casts.
- Dangling references from refused functions.
- Regressions: none.

```findings
- file: adapters/go/cmd/codespec-write-go/sorted_keys.go
  line: 79
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: shadowedIn checks only the function's own locals, so a package-level function named make, len or append (a Java package-private static method, read with 0 refusals) shadows the builtin the sorted_keys closure calls and the writer emits Go that fails vet, exit 0
- file: adapters/rust/src/write.rs
  line: 2855
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the `<` parenthesisation tests only a left operand that is itself a cast, so (mask & int32(len(xs))) < n prints `mask & xs.len() as i64 as i32 < n` and the function is refused as internal instead of written
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 876
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: sprintf, errorf and println are written as fmt.X under the plain import name, so a Java parameter or local named fmt shadows it and the Go does not compile
```
