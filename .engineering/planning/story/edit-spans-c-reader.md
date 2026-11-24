---
format: aep.planning-md/3
id: story:edit-spans-c-reader
kind: story
status: draft
title: The C reader gives spans and reads through an overlay
summary: C reader spans from clang's JSON AST offsets; codespec edit --lang c
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
revision: 1
---
# Story: The C reader gives spans and reads through an overlay

## Outcome

`codespec-read-c --spans` and `--overlay`, with the roles and completeness of
`docs/code-edit-language.md` § 6.2, from the offsets of clang's JSON AST. `codespec edit --lang c`
runs the wave-1 operations. Takes over the C part of `story:reader-function-spans`.

## Done When

- The wave-1 operations on a checked-in C example give checked-in diffs, verify and compile.

## Scope

- `adapters/c-read`, `examples/edit`. Inferred.
