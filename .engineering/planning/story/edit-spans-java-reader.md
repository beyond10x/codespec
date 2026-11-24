---
format: aep.planning-md/3
id: story:edit-spans-java-reader
kind: story
status: draft
title: The Java reader gives spans
summary: Java reader spans; codespec edit --lang java
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
revision: 3
---
# Story: The Java reader gives spans

## Outcome

`codespec-read-java --spans`, with the roles and completeness of
`docs/code-edit-language.md` § 6.2. A constructor the reader names `New<T>` and other made names
are `lowered`. `codespec edit --lang java` runs the wave-1 operations, verified by re-reading a cache copy of the module root
(`docs/code-edit-language.md` § 7.3); the reader gets no overlay flag.

## Done When

- The wave-1 operations on the `ledger` example give checked-in diffs, verify and compile with
  `javac`.

## Scope

- `adapters/java`, `examples/edit`. Inferred.
