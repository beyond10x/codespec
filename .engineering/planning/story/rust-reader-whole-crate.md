---
format: aep.planning-md/3
id: story:rust-reader-whole-crate
kind: story
status: draft
title: The Rust reader reads a whole crate
summary: Follow mod declarations to file and inline modules; one cir/1 module per crate, positions naming each file
relations:
- decomposes: epic:codebase-transformations
revision: 1
---
# Story: The Rust reader reads a whole crate

## Outcome

`codespec-read-rust <crate dir>` reads every source file of the crate's library target (or of its
binary target when it has no library): it follows `mod` declarations to file modules (`foo.rs`,
`foo/mod.rs`, a `#[path]` attribute) and into inline `mod foo { … }` blocks, and prints one
`cir/1` module for the crate. Every position names the file it stands in, relative to the crate's
`src/` directory (`lib.rs:12`, `wire.rs:40`, `net/mod.rs:3`), so a crate of one file reads to the
same bytes as before.

## Why

The reader reads `src/lib.rs` only. A crate whose code lives in module files is mostly not read:
a workspace of 30 crates measured with 234 non-test source files gave 30 files read. Reading each
file as its own crate instead loses the names of its sibling modules (more refusals) and every
position names `lib.rs`.

## Acceptance

- A test crate with a file module, a `foo/mod.rs` module, a nested module of a module file and an
  inline module reads to one module holding the functions of all of them; each refusal and each
  statement position names its own file and line.
- A `mod` whose file is missing is a refusal at the `mod` line, not an error; a `#[cfg(test)]`
  module is not read; a module file is read once even when two `#[path]` attributes name it.
- Two items of one kind and one name in two modules are each refused at their line, naming both
  modules, and no use of the name resolves to either: the reader resolves names by their last
  segment, and the module stays valid `cir/1`.
- The size and nesting limits hold per file; a crate of more than a stated number of files is
  refused with exit 1 and a message.
- Every checked-in example and conformance case reads to the same bytes (`task examples-check`,
  `task conformance`); `cargo test -p codespec-read-rust` exits 0.

## Scope

- Own: `adapters/rust-read`. Inferred.
