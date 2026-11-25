---
format: aep.planning-md/3
id: story:rust-reader-refused-signature-keeps-body
kind: story
status: draft
title: The Rust reader keeps a function's statements when its signature is refused
summary: A function whose signature cir/1 refuses keeps its body statements in the fact base, read or as Stmt::Foreign with reason and calls.
relations:
- depends_on: story:rust-reader-refused-bodies-list-calls
- decomposes: epic:codebase-transformations
revision: 1
---
## Outcome

A function whose signature `cir/1` refuses (a `&mut T` parameter, a bound that binds an associated
type such as `I: IntoIterator<Item = (K, V)>`, a type the reader cannot map) keeps its body: each
statement of the body is in the fact base as a statement that is read, or as a `Stmt::Foreign`
with its reason and its calls (the form `story:rust-reader-refused-bodies-list-calls` gives a
refused statement). The signature's refusal still names its construct, and the function stays
refused whole. A bound `cir/1` cannot state (the associated-type binding) stays refused naming the
bound.

## Why

Refs codesec#14 item 3, which confirms 4 defects in source: "For 4 defects confirmed in source, the
file was read but the statement was not in the fact base, because the enclosing item was refused".
They are a function taking `&mut T` ("a mutable reference `&mut T` is outside cir/1"), a function
with the bound `IntoIterator<Item = (K, V)>` ("binds an associated type, which cir/1 has no form
for"), a function using a type "the reader could not map", and an impl generated inside
`macro_rules!` (0 functions read in that file; that one is `story:rust-reader-trait-and-macro-items`).
Neither the codesec rules nor 12 rules written for these causes matched any of the 4. Reproduced
with a crate written from scratch:

```rust
pub fn helper(x: i64) -> i64 { x * 2 }
pub fn raise(c: &mut i64) -> i64 { helper(*c) + 1 }
pub fn total<I: IntoIterator<Item = (String, i64)>>(pairs: I) -> i64 {
    let mut t = 0; for (_, v) in pairs { t += v; } t
}
```

Both `raise` and `total` are refused whole with the messages above; the statements of their bodies
are not in the fact base.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: for `raise` and `total`, the fact
  base holds the body's statements (the call of `helper` in `raise`, the `for` loop in `total`),
  and each function is still refused naming its signature construct; red on main.
- The three-line crate above reads without a panic and without losing `helper`, which stays read.

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/refused-signature/` (not committed).
