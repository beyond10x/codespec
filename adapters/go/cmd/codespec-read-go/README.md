# codespec-read-go

The Go reader: reads one Go package directory, type-checked with `go/types`, and prints it as a
`cir/1` module on stdout. A construct `cir/1` cannot express is kept as a `foreign` statement with
the Go source and a refusal at its line.

```console
codespec-read-go <package dir>
codespec-read-go --describe
```

`--describe` prints the manifest (`describe.go`). The reader claims every feature whose
conformance kit case it reads, `stmt.foreign` among them (a `defer` kept verbatim). Facts are
computed by `codespec pass` and never claimed by a reader.

## Limits

The reader sets no limit of its own; its libraries bound depth, and a source past a bound is
refused with exit 1, not a crash. `go/parser` refuses nesting past 100 000 levels
(`maxNestLev`), the levels a chain of binary operators builds counted too
(`codespec-read-go: <file>:<line>:<col>: exceeded max nesting depth`). `encoding/json` refuses
to write a module nested past 10 000 levels (`maxNestingDepth`), which a chain of 99 000
operators the parser takes builds (`codespec-read-go: exceeded max depth`). Below those, Go's
stacks grow as the reader recurses, up to the runtime's 1 GB. Size is not limited.
`bounds_test.go` holds both.

## Features left unclaimed

The reader does not read the kit case of these six to its feature. For the first four and
`stmt.raise` the case is written by hand from another reader's output
(`crates/conformance/src/lib.rs` § Cases written by hand).

| feature | why |
|---|---|
| `decl.union` | Go has no tagged union |
| `expr.zero` | the reader reads a zero value as the Go source writes it, a literal (`0`, `""`, `false`), a struct literal that sets no field, `var x T`, so the kit case does not read to `zero`; it writes `zero` only for `struct{}{}` |
| `intrinsic.sorted_keys` | the reader reads `slices.Sorted(maps.Keys(m))` as `sorted_keys` over integer keys only: Go orders strings by byte, `sorted_keys` by UTF-16 code unit. The kit case's Go source collects the keys in a loop and sorts them with `sort.Strings` and `sort.Ints`, which the reader does not read as `sorted_keys` |
| `type.unit` | the kit case is a call of a function without results, which Go gives no type; the reader types it as an empty tuple. `struct{}` reads as `unit` |
| `stmt.raise` | Go has no statement that leaves a function, and every caller up to the framework that started it, with a value of another package's type; the case's Go source panics there, which the reader does not read as `raise` |
| `stmt.lookup_else` | the reader writes a comma-ok `let` and an `if`; only the lookup-guard pass of `codespec pass` makes them a `lookup_else`, and a reader's claim covers only what the reader prints (`docs/conformance.md` § 5) |

## Omitted

Two declarations are omitted without a refusal: they declare nothing a body can name and do
nothing when the package is initialized.

- A package-level blank variable with no initializer, or with one that has no run-time effect: a
  constant, `nil`, a function literal, a conversion of such a value, a composite literal of such
  values or its address (`var _ io.Writer = (*T)(nil)`, `var _ io.Writer = T{}`, the compiler's
  check that a type implements an interface). `var _ = f()` runs `f` and stays refused.
- A type alias, `type A = B`: every use of `A` reads as `B`.

## Read by lowering

- `_ = e` reads as `let _ = e`; an assignment with blank and named targets, `a, _ = f()`, as
  `let a_1, _ = f()` followed by `a = a_1`, when every named target is an identifier. A fresh name
  `<name>_<n>` is spelled nowhere else in the declaration being read.
- An `if` or `switch` with an init statement reads as the init in front of it (cir/1 has no block
  statement); a name the init declares is read under a fresh name where its spelling occurs
  elsewhere in the function, and `else if init; c` as an `else` holding the init and the `if`. A
  function or literal holding a `goto` keeps every such statement foreign, and so does an init or
  condition outside `cir/1`.
- Named results in a function or literal without a `defer` read as locals declared at the top,
  `var n T`, a blank result as `var r_<n> T`; a bare `return` returns them. With a `defer`, which
  can read or set them after the return, the body is kept in a foreign closure as before.

## Refused

- A range with `=` that assigns a variable declared before the loop (`for i = range xs`,
  `for s.f = range xs`), labelled or not, is kept foreign: a `cir/1` `for_range` declares its key
  and value, so it would shadow the variable and never assign it. A range that assigns only `_`,
  in parentheses or not, is read.
- A call passed the results of a multi-valued call, `f(g())`, is kept foreign: `cir/1` has no
  argument that spreads a tuple over parameters.
