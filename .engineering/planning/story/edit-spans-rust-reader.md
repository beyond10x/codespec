---
format: aep.planning-md/3
id: story:edit-spans-rust-reader
kind: story
status: active
title: The Rust reader gives spans
summary: decl and name spans and use refs from the Rust reader, in source spelling, paths threaded through its builders
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: adapters/rust-read
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:48Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:45Z", actor: "human:timo", revision: 8}
---
# Story: The Rust reader gives spans

## Outcome

`codespec-read-rust --spans <dir>` prints the object of `docs/code-edit-language.md` § 6.3. The
model's types refuse extra members, so the reader threads each node's `at` path through its
builders (`body.rs`, `calls.rs`, `lib.rs`) and records a span where it places a node in its parent;
a node it moves or makes is `lowered`. Roles and completeness as for the Go reader: `decl` for
every function (from its first doc comment or attribute to its end), `name` for every function,
struct field, call, method call, function value, field access and struct literal field, `text`
the identifier as written in Rust (the source spelling, not the Go spelling `cir/1` holds). A
struct literal field in shorthand (`T { x }`) carries `form: "shorthand"`. Every `use` path
segment that names an item of the crate is a `refs` record with its target's id; an `as` alias is
outside the span. Offsets come from `proc_macro2::Span::byte_range()` (`span-locations` is on).

## Done When

- `adapters/rust-read/tests/reader.rs` (the existing target), with `b10x-codespec-edit` added as a
  dev-dependency for its strict reading: for `examples/inventory` the spanned read passes strict
  reading and its `cir/1` is byte-identical to the checked-in
  `inventory.read.cir.json`; the output object carries `files`.
- Named cases with expected span documents checked in: a `pub fn load` (`text` `load`, `cir/1`
  name `Load`); a method in an `impl`; a function with attributes and doc comments; a shorthand and
  a keyed struct literal field; `use crate::m::load;` and `use crate::m::load as fetch;` (the `refs`
  record covers `load` only, a call `fetch()` has `name` text `fetch`); a `?` expression
  (`lowered`); CRLF line ends; a byte-order mark; a raw identifier `r#type`.
- No test over 5 s.

## Critic findings answered

- Round 2, strict reading needs `crates/edit` but the reader has no dev-dependency on it, and
  `Cargo.lock` has two writers: the dev-dependency is named, and this story is the only wave-1
  writer of `Cargo.lock` after `story:edit-spans-format`.

- How spans attach to model types that refuse unknown fields (parallel safety): the reader computes
  `at` paths itself and prints spans beside the module, never inside it.
- `--describe` spans mark untested (acceptance): no mark; the output object is tested.
- Overlay (design): none; re-reading uses a cache copy.

## Scope

- Own: `adapters/rust-read/src/` (`main.rs` flag, `lib.rs`, `body.rs`, `calls.rs`, `files.rs`, a
  new `spans.rs`), `adapters/rust-read/tests/reader.rs` and its fixtures.
- `adapters/rust-read/Cargo.toml` (the dev-dependency) and `Cargo.lock`: the one `Cargo.lock` change
  after `story:edit-spans-format`; `story:edit-engine-rename`, beside it, adds no dependency and does
  not touch `Cargo.lock`.
- Draft Rust reader stories (`story:rust-reader-casts-slices-unsafe`,
  `story:rust-reader-open-without-blocking`) land on the same files; none is active.
