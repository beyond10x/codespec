# codespec-read-go

The Go reader: reads one Go package directory, type-checked with `go/types`, and prints it as a
`cir/1` module on stdout. A construct `cir/1` cannot express is kept as a `foreign` statement with
the Go source and a refusal at its line.

```console
codespec-read-go <package dir>
codespec-read-go --describe
```

`--describe` prints the manifest (`describe.go`). The reader claims every feature of the
conformance kit that Go source reaches, `stmt.foreign` among them (a `defer` kept verbatim). Facts
are computed by `codespec pass` and never claimed by a reader.

## Features left unclaimed

Go source does not reach these five; another reader produces each, and its kit case is written by
hand (`crates/conformance/src/lib.rs` § Cases written by hand):

| feature | why |
|---|---|
| `decl.union` | Go has no tagged union |
| `expr.zero` | the reader writes a literal (`0`, `""`) or a struct literal that sets no field |
| `intrinsic.sorted_keys` | Go has no expression for a map's keys in order |
| `type.unit` | a call of a function without results is typed as an empty tuple |
| `stmt.lookup_else` | the reader writes a comma-ok `let` and an `if`; only the lookup-guard pass of `codespec pass` makes them a `lookup_else`, and a reader's claim covers only what the reader prints (`docs/conformance.md` § 5) |

## Refused

A range with `=` that assigns a variable declared before the loop (`for i = range xs`, `for s.f =
range xs`) is kept foreign: a `cir/1` `for_range` declares its key and value, so it would shadow the
variable and never assign it. A range that assigns only `_` is read.
