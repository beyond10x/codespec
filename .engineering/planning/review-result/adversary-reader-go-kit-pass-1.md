---
format: aep.planning-md/3
id: review-result:adversary-reader-go-kit-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:reader-go-kit'
relations:
- reviews: story:reader-go-kit
revision: 1
---
unit: story:reader-go-kit at commit 44ed324 (base fef8829), plus my uncommitted test files in /home/timo/.local/state/worktree/trees/b10x/codespec/rd-go-kit
verdict: CONFIRMED
cases: executed 62→67, red 3
origin: introduced 5 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, /home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/go-kit/adv1
needs-coordinator: none

No blockers. The unit's own claims hold: the range refusal is right on every shape I tried, none of the listed type assertions could be made to panic, and the reader never prints any of the five unclaimed features. Three new cases are red: one existing defect in how tuples are passed as arguments, and two small range edge cases.

**1. Diff.** `git --no-pager diff --stat` is empty because my files are untracked. `git status --short`:
```
?? adapters/go/cmd/codespec-read-go/adversary2_test.go
?? adapters/go/cmd/codespec-read-go/testdata/
```
Only test files and fixtures (`testdata/adversary2/tuplearg/p.go`, `testdata/adversary2/rangeforms/p.go`). No implementation file was touched.

**2. Cases added** (`adversary2_test.go`), with the run of them alone (`go test -count=1 -v -run TestAdversary2`, exit 1):
```
--- FAIL: TestAdversary2ACallPassedAMultiValuedCallIsReadAsItsArgumentsOrRefused
    adversary2_test.go:37: Spread: the call of add has 1 argument(s), one of them of type tuple, and the reader refuses nothing (refusals []): Go passes each result as its own argument
    adversary2_test.go:37: Variadic: the call of sum has 1 argument(s), one of them of type tuple, and the reader refuses nothing (refusals []): ...
--- FAIL: TestAdversary2ARangeAssigningOnlyAParenthesisedBlankIsRead
    adversary2_test.go:50: ParenBlank: body kinds [let foreign return] refusals [{p.go:6 for (_) = range xs { … range with = assigns variables declared before the loop; ...} ...]
--- FAIL: TestAdversary2ALabelledRangeThatAssignsAnOuterVariableIsRefusedForThatReason
    adversary2_test.go:64: Labelled: refused at p.go:15 for "labeled statement other than a loop whose label only its own break or continue names", want the range-assigns reason
--- PASS: TestAdversary2TheManifestLeavesEachOfTheFiveUnclaimed   (kills mutant M6)
--- PASS: TestAdversary2TheTypeTupleCaseBindsACallOfATupleOfTwoInts (catches M5 directly)
```

**3. Suite**, run after the cases existed:
- `cd adapters/go && go test -count=1 -v ./cmd/codespec-read-go/`: 67 top-level tests, exit 1, the 3 FAILs above.
- The same suite in a scratch copy with `adversary2_test.go` removed: 62 tests, `ok ... 6.643s`, exit 0.

**Mutants** (each in a scratch copy; "unit suite" means without my file):

| mutant | unit suite | my cases |
|---|---|---|
| M1 `rangeName(nil)` returns `""` | killed (TestARangeThatAssignsOnlyBlanksIsRead) | killed |
| M2 a non-name returns `"_"` | killed (…AssignsAFieldIsRefused…) | killed |
| M3 check the key only | killed (…DeclaredBeforeIt…) | killed |
| M4 `Tok != DEFINE` instead of `== ASSIGN` | survives | survives: equivalent mutant, `for range` has no key and no value |
| M5 a tuple typed as its first element | killed only by a crash in TestAStandardPackage…; the new type.tuple test does not catch it | TheTypeTupleCaseBinds… kills it |
| M6 `describe()` lists only `stmt.lookup_else` as not claimed | **survives** | TheManifestLeavesEachOfTheFive… kills it |
| M7 `rangeName` ignores parentheses (the fix for F3) | stays green | makes F3's case pass |

**4. Findings**

**F1 (warning, CONFIRMED, pre-existing): `f(g())` is read as one argument of tuple type.**
- `front.go:1228` (`call`). `add(two())` and `sum(two())` become a call with 1 argument of type tuple, with no refusal. `codespec validate` accepts the module.
- Both writers then refuse those functions: Rust says `"a call of add with 1 arguments for 2 parameters"`, C says `"a call whose arguments do not match the parameters"`.
- Go: `Spread 3`, `Variadic 3` from `go run`.
- The base reader prints the same thing (`{"k":"call","name":"add","nargs":1,"argt":"tuple"}`), so the defect is older than the unit. The unit's new `type.tuple` claim now covers this shape.
- What reaches it: any Go code that passes a call's results straight into another call.
- Suggested fix: read it as the separate results, or keep the statement foreign.

**F2 (warning, CONFIRMED, introduced in the new README): the reason given for `intrinsic.sorted_keys` is false.**
- `README.md:24` and `describe.go:14` say Go has no expression for a map's keys in order. Since Go 1.23 it does:
  - `slices.Sorted(maps.Keys(m))` printed `[a b c]` under `go run`.
  - The reader refuses it (`expression outside cir/1`).
- So "not reachable from Go source" is true of this reader but not of Go. The same wording already stood in `crates/conformance/src/lib.rs:39`, which the story's acceptance told the unit to copy.

**F3 (note, INFEASIBLE, introduced): `for (_) = range xs` is refused.**
- `front.go:572`. It contradicts the README line "A range that assigns only `_` is read."
- Go: `ParenBlank 3`. The base reader panicked here, so this is an improvement, just not what the page says.
- What reaches it: nothing found; nobody writes this. Fix: unwrap the parentheses (M7).

**F4 (note, CONFIRMED, introduced): a labelled `L: for i = range xs { break L }` is refused for the wrong reason.**
- `front.go:743` (`labeled`). The refusal says "labeled statement other than…", when the real cause is the range assignment.
- The base reader read this loop as a for_range that declares its key, so the outer variable was never assigned (Go: `Labelled 1`).

**F5 (note, CONFIRMED, introduced): `rangeName` was inserted between `foreign`'s doc comment and `foreign`.**
- `front.go:563-575`. The comment now documents `rangeName`, and `foreign` has none.

**F6 (note, CONFIRMED, introduced): the unit's `type.tuple` test cannot catch two defects.**
- `describe_test.go:79` compares the `claimNot` variable with a literal, so M6 survives it.
- Its check of the type-tuple case only asserts there are no refusals, so it does not catch M5.
- Whether `task conformance` catches M6 was not run.

**5. Attacked and not broken**
- **Range with `=` assigning an outer variable** is refused at its line for every shape tried: `i, v`, `_, v`, over an int, a string, a map, an array pointer, a function iterator, inside a closure, `(i)`, `a[0]`, `*p`. `for range xs` and range-over-func with blanks are read.
- **No panics.** These were all read with exit 0:
  - blank type, const, var and func declarations, and `init`
  - method values on `err.Error`, a type parameter, a constraint with a type union, a pointer-type parameter, a generic interface, an embedded interface and a promoted pointer method
  - method expressions, generic method values, type aliases and generic aliases
  - conversions that look like calls, the builtins `min`, `max`, `clear`, `new`, `copy`, and `len` of an array pointer
  - parenthesised receivers and callees, explicit instantiation used as a value, struct literals with embedded fields and generic types, a literal of a type-parameter struct
- **Other tuple shapes:** three results, named results (kept foreign), `return g()`, `return h()` through a function value, a generic tuple, `_, b := g()`, and a `:=` that redeclares a variable read correctly; the C writer gives the Go result 12. `c, _ = g()` is refused.
- **The five unclaimed features:** the reader has no code path that prints `union`, `zero`, `sorted_keys`, `unit` or `lookup_else`. The `reader-go.json` summary is pass 115, fail 0, pass_unclaimed 0 (it was 114 and 1).

**6. Written outside the worktree:** /home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/go-kit/adv1. It holds the probe packages (`rng`, `tup`, `tup2`, `tup3`, `sk`, `lab`, `pan/*`), the scripts (`mk.sh`, `mk2.sh`, `panics.sh`, `mutants.sh`, `sum.jq`), the binaries `reader` and `reader-base`, a base source tree `base/`, the mutant copies `mut/`, the symlink `conformance` pointing into the worktree, the writer outputs `out-*` and `o3-*`, and the `*.json` and `*.log` files.

**7.**
```findings
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 1228
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: f(g()) with a multi-valued g is read without refusal as a call with one tuple-typed argument, which cir/1 does not define and both writers refuse as an arity mismatch, now under the type.tuple claim
- file: adapters/go/cmd/codespec-read-go/README.md
  line: 24
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the stated reason that Go has no expression for a map's keys in order is false since Go 1.23 (slices.Sorted(maps.Keys(m))), which the reader refuses rather than reads
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 572
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: for (_) = range xs is refused although the page says a range assigning only blanks is read, because rangeName does not unwrap parentheses
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 743
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a labelled range assigning an outer variable is refused with the labeled-statement reason instead of the range-assigns reason, because labeled() replaces the inner refusal
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 563
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: rangeName was inserted between foreign's doc comment and foreign, so the comment now documents rangeName and foreign has none
- file: adapters/go/cmd/codespec-read-go/describe_test.go
  line: 79
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the new test pins the claimNot variable and checks only for no refusals, so a manifest that drops four of the five and a call typed int instead of tuple both stay green in the Go suite
```
