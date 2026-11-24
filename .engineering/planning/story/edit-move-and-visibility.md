---
format: aep.planning-md/3
id: story:edit-move-and-visibility
kind: story
status: draft
title: move a function to another module, and set_visibility
summary: move with qualifiers and imports; visibility the language's way
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-imports
revision: 1
---
# Story: move a function to another module, and set_visibility

## Outcome

`move(F, module)` moves a function's declaration to a file of another module of the set, rewrites
the qualifier at every call site and the imports both ways, and exports it when a caller is left
outside its new module. `set_visibility(F, v)` changes visibility in the language's own way: Go
renames to the upper- or lower-case spelling with every reference, Rust adds or removes `pub`.

## Done When

- Examples 9 and 10 of `docs/code-edit-language.md` § 3.2 on the Go and Rust examples give
  checked-in diffs, verify and build; a move that would create an import cycle is refused.

## Scope

- `crates/edit`, `examples/edit`. Inferred. Needs `story:edit-imports`.
