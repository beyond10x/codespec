---
format: aep.planning-md/3
id: story:edit-engine-rename
kind: story
status: implemented
title: 'crates/edit: change sets, rename and rename_field'
summary: 'The edit crate: expected cir/1, text edits, refusals, comparison and diff; rename and rename_field'
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: crates/edit
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:46Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-24T02:58:37Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
# Story: crates/edit: change sets, rename and rename_field

## Outcome

`crates/edit` gains what `codespec edit` does between the rows and the files, with no process,
file system or adapter call:

- A change set: operations in row order, each giving (1) the expected set `S'` on `cir/1`
  (`docs/code-edit-language.md` § 5) and (2) text edits `(file, start, end, replacement)` from the
  span documents. Byte-identical edits merge; overlapping different ones refuse the set. Edits are
  sorted by path, then start, and applied from the end; a line shift per file moves every later
  `pos` of `S'`.
- `rename(F, New)`: the declaration, every read reference, every `extern`/`extern_method` of another
  module, and every Rust `use` path segment (`refs`) naming `F`. A `name` span whose `text` is an
  alias (a call `fetch()` after `use … load as fetch`) is left alone. For a method, `S'` renames its
  `MethodEntry` (`name` and `target`) and sorts the method set by name again.
- `rename_field(T, Old, New)`: declaration, accesses and keyed literal fields; Rust shorthand
  renders as `new: old`.
- Every refusal of § 7.1 that applies to them, computed in Rust over the `cir/1` set and the span
  documents (the fact base has no `foreign` `src`; this crate does not depend on `crates/query` at
  run time): a missing or `lowered` span; a reference in a listed call, in a `named` call ref of the
  same spelling, or in a `foreign` `src` holding the spelling as a word; a module whose reader fills
  no call lists; an interface method; an exported target without `closed_world`; a new name that is
  no identifier or a keyword of the language (Go and Rust tables), changes Go exportedness, or
  collides with a declaration of the module or a local or parameter of a referencing body; an edit
  in a `generated` file.
- Comparing a re-read set with `S'`: equal, or the first differing JSON path with both values. For
  a rename, `o'` is taken from the re-read declaration and must be used everywhere and nowhere
  else.
- A unified diff made from the edits: hunks around each edited span with three lines of context,
  `--- a/<path>` and `+++ b/<path>`.

## Done When

`crates/edit/tests/main.rs`, on hand-written `cir/1` and span fixtures (no adapter):

- rename of a function called from its module and from a second module via `extern` gives the
  expected `S'` and edits; rename of a Rust-spelled function (`cir/1` `Load`, `text` `load`) edits
  `load`;
- Rust `use` paths: with `use crate::m::load;` the `use` segment and the calls are edited; with
  `use crate::m::load as fetch;` the `use` segment is edited and the call `fetch()` is not;
- rename of a method: `S'` has the renamed `MethodEntry` and the method set in name order;
- rename_field over a declaration, an access, a keyed and a shorthand literal;
- one test per refusal above, each naming its site;
- two rows renaming one function to the same name merge, to two names refuse;
- comparison: `S'` against an equal set gives equal; against a set with one changed name gives that
  path and both values; a re-read using two different `o'` for one rename is a difference;
- the diff of a two-file rename equals a checked-in file, and `git apply --check` accepts it (the one
  test that runs a process; it fails, not skips, when `git` is absent);
- one change set run twice gives the same bytes.

No test over 5 s; no Python.

## Critic findings answered

- Rust `use` path refs are not edited (scope, blocker): outcome and the two `use` cases above.
- Refusals said to be cq rules in a crate without a query dependency (design): they are Rust over
  `cir/1` and spans; the design § 7.1 says so.
- Method rename misses `MethodEntry` and the sort (design): in the outcome and a case.
- The re-read comparison is untested (acceptance): the three comparison cases.

## Scope

- Own: `crates/edit/src/` (`change.rs`, `rename.rs`, `refs.rs`, `lang.rs`, `compare.rs`,
  `diff.rs`, `lib.rs`), `crates/edit/tests/main.rs`, `crates/edit/tests/fixtures/`.
- Runs after `story:edit-spans-format` (the crate, its span types and its dependencies). Adds no
  dependency and does not touch `Cargo.toml` or `Cargo.lock` (`story:edit-spans-rust-reader`, beside it,
  writes `Cargo.lock`). Declares its modules in `crates/edit/src/lib.rs`; `story:edit-engine-delete-retarget`
  extends the crate after it. Does not touch `src/run.rs`.
