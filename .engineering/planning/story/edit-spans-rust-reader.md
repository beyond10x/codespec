---
format: aep.planning-md/3
id: story:edit-spans-rust-reader
kind: story
status: proposed
title: The Rust reader gives spans and reads through an overlay
summary: decl, node, name spans and use refs from the Rust reader, in source spelling, and --overlay
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: adapters/rust-read
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:48Z", actor: "human:timo", revision: 3}
---
# Story: The Rust reader gives spans and reads through an overlay

## Outcome

`codespec-read-rust --spans <dir>` prints a spanned module (`docs/code-edit-language.md` § 6.3)
with the same roles and completeness as the Go reader: `decl` for every function and type (from
its first doc comment or attribute to its end), `node` for every statement, `name` for every
function, struct field, call, method call, function value, field access and struct literal field,
`text` the identifier as written in Rust (the source spelling, not the Go spelling `cir/1` holds).
A struct literal field in shorthand (`T { x }`) carries `form: "shorthand"`. Every `use` path
segment that names an item of the crate is a `refs` record with its target's id. Nodes the reader
makes (a `?` read as `if err != nil`, a renamed shadowing local) are `lowered`. Offsets come from
`proc_macro2::Span::byte_range()` (`span-locations` is on already).

`codespec-read-rust --overlay <file> <dir>` reads overlay contents in place of the named files.
`--describe` marks the reader as giving spans.

## Done When

- `adapters/rust-read/tests/reader.rs` (the existing target): for `examples/inventory` the spanned
  read passes strict reading and its `cir/1` is byte-identical to the checked-in
  `inventory.read.cir.json`.
- Named cases with expected span documents checked in: a `pub fn load` (`text` `load`, `cir/1`
  name `Load`); a method in an `impl`; a function with attributes and doc comments; a shorthand and
  a keyed struct literal field; `use crate::m::load;` and `use crate::m::load as fetch;` (the `refs`
  record covers `load` only); a `?` expression (`lowered`); CRLF line ends; a byte-order mark; a
  raw identifier `r#type`.
- An overlay case: a crate whose `lib.rs` is replaced in the overlay reads the replacement.
- No test over 5 s.

## Scope

- Own: `adapters/rust-read/src/` (`main.rs` flags, `files.rs` file loading and overlay, `lib.rs`,
  `body.rs` and `calls.rs` where nodes are built, a new `spans.rs`), `adapters/rust-read/tests/reader.rs`
  and its fixtures.
- Draft stories of the Rust reader (`story:rust-reader-casts-slices-unsafe`,
  `story:rust-reader-open-without-blocking`) land on the same files; none is active now.
