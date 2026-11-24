---
format: aep.planning-md/3
id: story:edit-imports
kind: story
status: draft
title: Imports in edits, and retarget to another module
summary: Import ranges in spans; ensure_import, remove_import; cross-module retarget
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
revision: 2
---
# Story: Imports in edits, and retarget to another module

## Outcome

Span documents carry each file's import range (Go import block, Rust `use` items; an insertion
offset when a file has none). `crates/edit` gains `ensure_import` and `remove_import`, and a
`retarget` to a function of another module adds the import it needs and removes one it left unused.
Limited to these: the imports a template needs are `story:edit-replace-template`'s, the imports of a
moved function `story:edit-move-and-visibility`'s, both on this story's import ranges. Takes over
the import-range part of `story:reader-function-spans`.

## Done When

- A cross-module `retarget` on the Go and Rust examples adds the import, verifies and builds; a
  retarget away from the last use of an import removes it.

## Scope

- `adapters/go/cmd/codespec-read-go`, `adapters/rust-read`, `crates/edit`, `examples/edit`.
  Inferred.
