---
format: aep.planning-md/3
id: story:edit-engine-rename
kind: story
status: proposed
title: 'crates/edit: change sets, rename and rename_field'
summary: 'The edit crate: expected cir/1, text edits, refusals, comparison and diff; rename and rename_field'
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/edit
- confidence: inferred
  path: scripts/check-parts.txt
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:49Z", actor: "human:timo", revision: 3}
---
# Story: crates/edit: change sets, rename and rename_field

## Outcome

A new crate `crates/edit` (`b10x-codespec-edit`, library only) holds what `codespec edit` does
between the rows and the files, with no process, file system or adapter call:

- A change set: operations in row order, each giving (1) the expected set `S'` on `cir/1`
  (`docs/code-edit-language.md` § 5) and (2) text edits `(file, start, end, replacement)` from the
  span documents. Byte-identical edits merge; overlapping different ones refuse the set. Edits are
  sorted by path, then start, and applied from the end; a line shift per file moves every later
  `pos` of `S'`.
- `rename(F, New)` and `rename_field(T, Old, New)` with every refusal of § 7.1 that applies to them:
  a missing or `lowered` span, a reference in a listed call, a `named` call ref or a `foreign` `src`
  holding the source spelling as a word, a module whose reader fills no call lists, an interface
  method, an exported target without `closed_world`, a new name that is no identifier or a keyword
  of the language (Go and Rust tables), changes Go exportedness, or collides with a declaration of
  the module or a local or parameter of a referencing body; an edit in a `generated` file. Rust
  shorthand fields render as `new: old`.
- Comparing a re-read set with `S'`: equal, or the first differing JSON path with both values. For
  a rename, `o'` (§ 5) is taken from the re-read declaration and must be used everywhere and
  nowhere else.
- A unified diff made from the edits: hunks around each edited span with three lines of context,
  `--- a/<path>` and `+++ b/<path>`.

## Done When

- `crates/edit/tests/main.rs` (one test target), on hand-written `cir/1` and span fixtures (no
  adapter, no process): rename of a function called from its module and from a second module via
  `extern` gives the expected `S'` and edits; rename of a Rust-spelled function (`cir/1` `Load`,
  `text` `load`) edits `load`; rename_field over a declaration, an access, a keyed and a shorthand
  literal; one test per refusal above, each naming its site; two rows renaming one function to the
  same name merge, to two names refuse; the diff of a two-file rename equals a checked-in file and
  `git apply --check` accepts it (the one test that runs a process, skipped with a message when
  `git` is absent).
- Every function of `crates/edit` is deterministic: a test runs one change set twice and compares
  bytes.
- `scripts/check-parts.txt` names the crate in a `test` part; `task check-parts` passes. No test
  over 5 s; no Python.

## Scope

- Own: `crates/edit/` (new: `Cargo.toml`, `src/lib.rs`, `src/change.rs`, `src/rename.rs`,
  `src/lang.rs`, `src/diff.rs`, `tests/main.rs`, `tests/fixtures/`), the workspace `Cargo.toml`
  (members), `Cargo.lock`, `scripts/check-parts.txt`.
- Needs the span types of `story:edit-spans-format`. `story:edit-engine-delete-retarget` extends the
  same crate after it.
