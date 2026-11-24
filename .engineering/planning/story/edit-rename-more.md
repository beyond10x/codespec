---
format: aep.planning-md/3
id: story:edit-rename-more
kind: story
status: draft
title: Rename types, locals, parameters and interface methods
summary: rename for the remaining name kinds, an interface method with its implementations
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
revision: 1
---
# Story: Rename types, locals, parameters and interface methods

## Outcome

`rename` accepts a type id (every type reference in signatures, literals, conversions and other
modules' `extern` types), a local or parameter (`<body>$<name>`, every use in its body), and an
interface method together with the same method of every implementing type in the set. Readers give
`name` spans of type references and locals.

## Done When

- Each kind renamed on the Go and Rust examples gives a checked-in diff, verifies and builds; a
  local renamed to a name that would capture another reference is refused.

## Scope

- `crates/edit`, `crates/model/src/spans.rs`, `adapters/go/cmd/codespec-read-go`,
  `adapters/rust-read`, `examples/edit`. Inferred.
