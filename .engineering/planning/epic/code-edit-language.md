---
format: aep.planning-md/3
id: epic:code-edit-language
kind: epic
status: proposed
title: 'Code edits in cq: a query selects, an operation changes the source'
summary: 'codespec edit: cq edit items, operations defined once on cir/1, span edits of the original files, verified by a re-read'
relations:
- informed_by: epic:codebase-transformations
revision: 2
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:47Z", actor: "human:timo", revision: 2}
---
# Epic: Code edits in cq: a query selects, an operation changes the source

## Outcome

`codespec edit` changes source code by a cq program: an `edit` item's body selects targets in
`cir/1` terms, its operation (`rename`, `rename_field`, `retarget`, `delete` in wave 1) is defined
once on `cir/1` for every language, and the change lands as byte-span edits of the original files,
keeping every byte outside the edited spans. Every change is shown as a unified diff first, refused
when it would touch code the reader did not read, and verified before it is written: the edited
files, read again through an overlay, must give exactly the `cir/1` the operation predicts. Go and
Rust first.

Design: `docs/code-edit-language.md`.

## Why Now

Timo, 2026-11-24: "a unified mutation language … the query would basically select the target to
what it should be applied, and then we run the operations against this limited filtered scope …
language agnostic, kind of." cq selects typed nodes already (`crates/query`), the readers resolve
names (`go/types`, `syn`), and the re-read that proves an edit is a `codespec read`. What is
missing is spans from the readers, the operations, and the command.

## Waves

1. Spans (`codespec-spans/1`, Go and Rust readers), `edit` items in cq, the operations in
   `crates/edit`, `codespec edit` with diff, verification and `--apply`, `--build` and `gofmt`,
   checked-in examples and the site page.
2. Imports and cross-module `retarget`; templates (`replace`, `replace_stmt`, insertions);
   `add_param`, `remove_param`.
3. Rename of types, locals, parameters and interface methods; `move`, `set_visibility`; C and Java
   readers giving spans.

## Done When

- Every wave-1 story of this epic is implemented.
- `task examples-edit` passes in the gate: on the checked-in Go and Rust examples, each wave-1
  operation's dry-run diff equals its checked-in `.diff`, the applied result is verified and
  builds, each refusal case exits 1 naming its site, and nothing is written on a refusal.

## Out of Scope

- Edits inside code a reader did not read (foreign statements, opaque bodies, macro insides).
- Text search and replace; edits of comments, strings and doc text.
- Re-formatting code outside the edited spans (Go's `gofmt` of a file that was `gofmt`-clean aside).
- A proof that a change keeps behaviour beyond the `cir/1` equality (the evaluator's differential
  run is `story:mcp-verify-change`).
- An editor server (LSP). `epic:agent-mcp-server` calls `crates/edit`.
