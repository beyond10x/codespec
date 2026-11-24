---
format: aep.planning-md/3
id: story:go-read-foreign-struct-literal
kind: story
status: draft
title: Go reader reads a struct literal of another package's type
summary: a sibling package's struct literal reads as the struct, not foreign
relations:
- decomposes: epic:reader-coverage
revision: 2
---
# Story: Go reader reads a struct literal of another package's type

## Outcome

The Go reader reads a composite literal whose type is a struct of another package of the module
(`stock.Item{Count: 1}`) as that struct, not as `foreign`, so `rename_field` can rename its field
there instead of refusing.

Found by `story:edit-examples-and-docs` (2026-11-24).

## Done When

- A Go reader test: a literal of a struct of a sibling package reads to the struct's cir/1 type with
  its field names; it fails before the change.
- `codespec edit` renames a field used in such a literal (CLI test in `crates/cli/tests/edit`), and
  verification passes.
