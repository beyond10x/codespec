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

## Features left unclaimed

The reader does not read the kit case of these six to its feature. For the first four and
`stmt.raise` the case is written by hand from another reader's output
(`crates/conformance/src/lib.rs` § Cases written by hand).

| feature | why |
|---|---|
| `decl.union` | Go has no tagged union |
| `expr.zero` | the reader never writes `zero`: it reads a zero value as the Go source writes it, a literal (`0`, `""`, `false`), a struct literal that sets no field, `var x T` |
| `intrinsic.sorted_keys` | the reader reads `slices.Sorted(maps.Keys(m))` as `sorted_keys` over integer keys only: Go orders strings by byte, `sorted_keys` by UTF-16 code unit. The kit case's Go source collects the keys in a loop and sorts them with `sort.Strings` and `sort.Ints`, which the reader does not read as `sorted_keys` |
| `type.unit` | Go gives a call of a function without results no type; the reader types it as an empty tuple |
| `stmt.raise` | Go has no statement that leaves a function, and every caller up to the framework that started it, with a value of another package's type; the case's Go source panics there, which the reader does not read as `raise` |
| `stmt.lookup_else` | the reader writes a comma-ok `let` and an `if`; only the lookup-guard pass of `codespec pass` makes them a `lookup_else`, and a reader's claim covers only what the reader prints (`docs/conformance.md` § 5) |

## Refused

- A range with `=` that assigns a variable declared before the loop (`for i = range xs`,
  `for s.f = range xs`), labelled or not, is kept foreign: a `cir/1` `for_range` declares its key
  and value, so it would shadow the variable and never assign it. A range that assigns only `_`,
  in parentheses or not, is read.
- A call passed the results of a multi-valued call, `f(g())`, is kept foreign: `cir/1` has no
  argument that spreads a tuple over parameters.
