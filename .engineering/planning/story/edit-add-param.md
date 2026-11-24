---
format: aep.planning-md/3
id: story:edit-add-param
kind: story
status: draft
title: add_param and remove_param at the declaration and every call site
summary: Change a signature and every call site, with a default argument
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
revision: 1
---
# Story: add_param and remove_param at the declaration and every call site

## Outcome

`add_param(F, index, name, type, default)` adds a parameter to `F` and the default, written in the
module's language, at every call site; `remove_param(F, index)` removes a parameter no body reads
and its argument at every call site whose argument has no effect. Readers give spans of parameter
lists and argument lists.

## Done When

- Example 8 of `docs/code-edit-language.md` § 3.2 on the Go and Rust examples gives checked-in
  diffs, verifies and builds; a call site inside unread code refuses the change.

## Scope

- `crates/edit`, `crates/model/src/spans.rs`, `adapters/go/cmd/codespec-read-go`,
  `adapters/rust-read`, `examples/edit`. Inferred.
