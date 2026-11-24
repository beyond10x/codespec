---
format: aep.planning-md/3
id: epic:code-edit-language
kind: epic
status: active
title: 'Code edits in cq: a query selects, an operation changes the source'
summary: 'codespec edit: cq edit items, operations defined once on cir/1, span edits of the original files, verified by a re-read'
relations:
- informed_by: epic:codebase-transformations
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:47Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:41Z", actor: "human:timo", revision: 4}
---
# Epic: Code edits in cq: a query selects, an operation changes the source

## Outcome

`codespec edit` changes source code by a cq program: an `edit` item's body selects targets in
`cir/1` terms, its operation (`rename`, `rename_field`, `retarget`, `delete` in wave 1) is defined
once on `cir/1` for every language, and the change lands as byte-span edits of the original files,
keeping every byte outside the edited spans. Every change is shown as a unified diff first, refused
when it would touch code the reader did not read, and verified before it is written: a cache copy
holding the edited files, read again, must give exactly the `cir/1` the operation predicts. Rust
and Go first.

Design: `docs/code-edit-language.md`.

## Why Now

Timo, 2026-11-24: "a unified mutation language … the query would basically select the target to
what it should be applied, and then we run the operations against this limited filtered scope …
language agnostic, kind of." cq selects typed nodes already (`crates/query`), the readers resolve
names (`go/types`, `syn`), and the re-read that proves an edit is a `codespec read`. What is
missing is spans from the readers, the operations, and the command.

## Waves

Wave 1 is one wave on the integration branch `wave/code-edit-language`, in this order (stories on
one line side by side):

1. `story:edit-spans-format`
2. `story:edit-cq-items`, `story:edit-spans-rust-reader`, `story:edit-engine-rename`
3. `story:edit-slice-rust-rename` (the first end-to-end proof: a Rust rename, dry run, verified
   re-read), `story:edit-engine-delete-retarget`
4. `story:edit-spans-go-reader`. **Hold:** the integration branch merges `origin/main` after the
   pull request of `wave/go-reader-refusals-1` is merged, and this story starts only then; that
   branch rewrites the Go reader's `front.go`.
5. `story:edit-cli-verified-apply`, then `story:edit-build-and-format`, then
   `story:edit-examples-and-docs`.

Later: wave 2, imports and cross-module `retarget` (`story:edit-imports`), templates
(`story:edit-replace-template`), `add_param` (`story:edit-add-param`); wave 3, more rename kinds,
`move` and `set_visibility`, C and Java spans.

## Done When

- Every wave-1 story of this epic is implemented.
- `task examples-edit` passes in the gate: on the checked-in Go and Rust examples, each of the four
  wave-1 operations' dry-run diff equals its checked-in `.diff` in both languages, the applied result
  is verified and builds, each refusal case exits 1 naming its site with every input file unchanged,
  and the task's self-test detects a changed expected diff.

## Out of Scope

- Edits inside code a reader did not read (foreign statements, opaque bodies, macro insides).
- Text search and replace; edits of comments, strings and doc text.
- Re-formatting code outside the edited spans (Go's `gofmt` of a file that was `gofmt`-clean aside).
- A proof that a change keeps behaviour beyond the `cir/1` equality (the evaluator's differential
  run is `story:mcp-verify-change`).
- Statement extents for a code quality assessor (`node` spans, an `extent` relation): that epic is
  not decided; the span format stays open to add them.
- An editor server (LSP). `epic:agent-mcp-server` calls `crates/edit`.
