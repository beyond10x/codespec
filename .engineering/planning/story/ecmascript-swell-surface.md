---
format: aep.planning-md/3
id: story:ecmascript-swell-surface
kind: story
status: draft
title: The reader holds swell's syntax surface at a pinned release
summary: A Git dependency on swell-syntax at a release tag, the list of public items the reader uses, a check against swell's promised surface, and span to position conversion.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:native-run-harness
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
  path: adapters/ecmascript-read/src/lib.rs
- confidence: inferred
  path: adapters/ecmascript-read/src/swell.rs
- confidence: inferred
  path: adapters/ecmascript-read/swell-surface.txt
- confidence: inferred
  path: adapters/ecmascript-read/tests/main.rs
- confidence: inferred
  path: adapters/ecmascript-read/tests/swell.rs
revision: 7
---
# Story: The reader holds swell's syntax surface at a pinned release

## Outcome

`adapters/ecmascript-read` depends on `swell-syntax` at one release tag of `beyond10x/swell`, and
every use of it goes through one module, `src/swell.rs`: the only file of the crate that names
`swell_syntax::`. That module re-exports the items the reader maps, holds `position(file, span)`
(a swell span as codespec's `<file>:<line>`) and the mapping of a swell `SyntaxError` to a refusal
(with the slice swell names). A committed list of the public swell items `swell.rs` names, a check
that the pinned swell still has them with the same shape, and a check that no other file names
`swell_syntax::` complete it. A swell release that changes what the reader uses fails a fast check
before it fails a build.

## Why

The reader is built on swell's parser (agreed with swell, 2026-11-25) and swell's tree is its
own: arenas `Ast` with `ExprId`, `StmtId` and `FunctionId`, accessors `Ast::expr`, `Ast::stmt`, a
`Span` of UTF-16 code unit offsets `start..end` plus `line` and `column` from 1
(`crates/syntax/src/ast.rs` in swell). codespec positions are `<file>:<line>`. swell is a library
with no binary and no published crate, so the dependency is a Git dependency on a tag. One module
holding every name of swell keeps the dependency's size one list.

## What codespec needs from swell

Not a story here: swell's work is swell's. The promise this story relies on is swell's draft
`story:syntax-surface-is-promised` (beyond10x/swell#9, branch `plan/native-code` in
`beyond10x/swell`):

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

- A failing check first: `ecmascript-read-surface` reads a test copy of the item list with one
  invented item and exits non-zero naming it; it fails until the check exists.
- `adapters/ecmascript-read/Cargo.toml` names `swell-syntax` by Git URL and tag, never a branch or
  a revision alone; a check refuses any other form. `Cargo.lock` holds the resolved commit.
- The one data source of the surface check is the checkout Cargo made of the pinned tag (the git
  checkout under `CARGO_HOME`, read offline after the build), whose `conformance/syntax/surface.txt`
  is compared with `adapters/ecmascript-read/swell-surface.txt`. Nothing is fetched by the check and
  no copy of swell's list is cached under `target/`.
- `swell-surface.txt` lists every swell public item `src/swell.rs` names (types, variants, fields,
  functions), generated from that file's `swell_syntax::` paths; the check fails on an item that
  is not in swell's list, and on a `swell_syntax::` path in any other source file of the crate.
- `position(file, span)` gives `<file>:<line>` for a span, and a test holds the cases that go wrong:
  a span after a surrogate pair (the column counts UTF-16 units, the offset too), a CRLF file, a
  file without a final newline, a span of length 0.
- A swell tag whose parser refuses a construct names its slice (`<construct> is not supported
  before slice <n>`); `src/swell.rs` maps that error to a refusal at the position the error gives,
  naming the slice. A test holds one such error (a class declaration, slice 8 at the time of
  writing).
- The checks run within the step budget with no network.

## Scope

- Own: `adapters/ecmascript-read/Cargo.toml` (a library target only), `swell-surface.txt`,
  `src/lib.rs` (minimal: `mod swell;` and its re-exports, so the crate builds and its tests run
  alone), `src/swell.rs`, `tests/main.rs` (one integration-test binary, declaring `mod swell;`),
  `tests/swell.rs` (all under `adapters/ecmascript-read/`, new files), `scripts/check-parts.txt`,
  root `Cargo.toml` and `Cargo.lock`, `Taskfile.yml`. `story:ecmascript-reader` extends `src/lib.rs`
  and `tests/main.rs`, adds `src/main.rs` and the other modules, and rewrites none of this
  story's files.
- Shares root `Cargo.toml`, `Cargo.lock`, `Taskfile.yml` and `scripts/check-parts.txt` with
  `story:native-run-harness`, which comes first; this story is before `story:ecmascript-reader`,
  which shares the crate directory.
