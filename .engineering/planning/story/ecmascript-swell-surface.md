---
format: aep.planning-md/3
id: story:ecmascript-swell-surface
kind: story
status: draft
title: The reader holds swell's syntax surface at a pinned release
summary: A Git dependency on swell-syntax at a release tag, the list of public items the reader uses, a check against swell's promised surface, and span to position conversion.
relations:
- decomposes: epic:ecmascript-to-native
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/ecmascript-read/Cargo.toml
- confidence: inferred
  path: adapters/ecmascript-read/swell-surface.txt
- confidence: inferred
  path: scripts
revision: 2
---
# Story: The reader holds swell's syntax surface at a pinned release

## Outcome

`adapters/ecmascript-read` depends on `swell-syntax` at one release tag of `beyond10x/swell`, and
codespec knows exactly which public items of it the reader uses: a committed list of them, a check
that the pinned swell still has them with the same shape, and one function that turns a swell span
into codespec's position form. A swell release that changes what the reader uses fails a fast
check before it fails a build.

## Why

The reader is built on swell's parser (agreed with swell, 2026-11-25) and swell's tree is its
own: arenas `Ast` with `ExprId`, `StmtId` and `FunctionId`, accessors `Ast::expr`, `Ast::stmt`, a
`Span` of UTF-16 code unit offsets `start..end` plus `line` and `column` from 1
(`crates/syntax/src/ast.rs` in swell). codespec positions are `<file>:<line>`. swell is a library
with no binary and no published crate, so the dependency is a Git dependency on a tag.

## What codespec needs from swell

Not a story here: swell's work is swell's. The promise this story relies on is swell's draft
`story:syntax-surface-is-promised` (branch `plan/native-code` in `beyond10x/swell`):

- the public items of `swell-syntax` listed in `conformance/syntax/surface.txt`, checked by
  `syntax-surface-check`; any public-item change is a minor version bump before 1.0;
- `ast::Span` on every node a reader maps (statements, expressions, functions, names, patterns,
  template parts), with the meaning above;
- a corpus `conformance/syntax/corpus.txt` of at most 200 test262 paths at test262 revision
  `2a1ade30` (BSD-3-Clause), one per ES5 statement and expression form, which this reader may
  take as its first cases;
- a release tag carrying all three. The tag is not known: this story's pin is the first swell
  release after `story:syntax-surface-is-promised`, and the story cannot be implemented before that
  release exists. It is not a story to write in this repository.

## Done When

- A failing check first: `ecmascript-read-surface` reports a name the pinned swell lacks (a test
  copy of the list with one invented item) and exits non-zero naming the item.
- `adapters/ecmascript-read/Cargo.toml` names `swell-syntax` by Git URL and tag, never a branch or
  a revision alone; a check refuses any other form. `Cargo.lock` holds the resolved commit.
- `adapters/ecmascript-read/swell-surface.txt` lists every swell public item the reader names (types,
  variants, fields, functions), generated from the crate's use of `swell_syntax::` paths; the
  check compares it with swell's `surface.txt` at the pinned tag (fetched once per run and cached
  under `target/`, no network in a test) and fails on an item that is not in swell's list.
- `position(file, span)` gives `<file>:<line>` for a span, and a test holds the cases that go wrong:
  a span after a surrogate pair (the column counts UTF-16 units, the offset too), a CRLF file, a
  file without a final newline, a span of length 0.
- A swell tag whose parser refuses a construct names its slice (`<construct> is not supported
  before slice <n>`); the reader reports it as a refusal at the position the error gives, naming
  the slice. A test holds one such error (a class declaration, slice 8 at the time of writing).
- The check runs within the step budget, with no network: the swell sources it reads come from
  the Cargo checkout, not a download.

## Scope

- Own: `adapters/ecmascript-read/Cargo.toml` and `swell-surface.txt` (new files; the crate itself
  is `story:ecmascript-reader`'s), `scripts/`, `Taskfile.yml`, `scripts/check-parts.txt`, root
  `Cargo.toml` `members` and `Cargo.lock`.
- Shares the crate directory, the root `Cargo.toml` and `Taskfile.yml` with
  `story:ecmascript-reader`: this story first.
