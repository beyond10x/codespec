---
format: aep.planning-md/3
id: story:cq-type-field-across-modules
kind: story
status: draft
title: cq keeps type_field facts when another module names the type
summary: type_field rows vanish when a second module of the set names the struct
relations:
- decomposes: epic:code-edit-language
revision: 2
---
# Story: cq keeps type_field facts when another module names the type

## Outcome

A cq query over a set of modules gives a `type_field` row for every field of a struct, also when
another module of the set names that struct (for example a function there returns it).

Found by `story:edit-examples-and-docs` (2026-11-24): with both Go packages of `examples/edit/`
loaded, `type_field{name: "Count"}` gave no row; with the declaring package alone it gave one. The
example was changed to avoid the cross-package use. The defect is in `crates/query`.

## Done When

- A test in `crates/query`'s integration-test binary loads two modules, the second naming a struct
  of the first, and gets one `type_field` row per field; it fails before the fix.
- `examples/edit/` gets back a cross-package use of a struct whose field a `rename_field` program
  renames.
