---
format: aep.planning-md/3
id: epic:typescript-reader
kind: epic
status: draft
title: A TypeScript reader for Node services
summary: codespec read --lang typescript gives functions, calls and http entries for the TypeScript components of a service release; parser choice is an open decision
relations:
- informed_by: epic:release-fact-diff
- informed_by: epic:tree-sitter-go-reader
- informed_by: epic:java-quarkus-reader
revision: 1
---
# Epic: A TypeScript reader for Node services

## Outcome

`codespec read --lang typescript <dir>` reads the TypeScript sources of a Node service into
`cir/1`, so that `codespec effects`, `reach` and `diff` give facts for the components of a release
that are written in TypeScript, as they do for Go: its functions, its calls (module functions,
imports as externs), and its HTTP routes as `http` entries. What the reader cannot express is
refused with a reason and `<file>:<line>`, never guessed: the smallest enclosing function becomes
an opaque function and the effects analysis reports it `unknown`, as for any `foreign` code.

## Why

A release review of a Go service ran `codespec diff` on two tags. The release holds ten
first-party components; six of them are TypeScript, and codespec gave no fact for any of them:
`epic:release-fact-diff` lists "TypeScript, Vue and Kamailio sources: codespec has no reader for
them" as out of its scope, and the repository has no reader for the language (`grep -ril
typescript` finds the site's UX harness only). The owner asked that every gap of the review be
reported upstream; this is the largest of them.

## What exists and is not duplicated

- `epic:ecmascript-to-native` (draft, branch `plan/ecmascript-native`, not on main when this was
  written) reads a typed ECMAScript subset into `cir/1` on swell's parser (`swell-syntax`), to
  write WebAssembly and x86-64 and to measure speed. It is not TypeScript (its decision D1: types
  come from JSDoc tags, "not TypeScript") and it targets a different program: numeric kernels with
  no `import`, no class, no `async`, no `try`, each of which a Node service uses. What this epic
  shares with it: the `cir/1` mapping of `number` (a float of 64 bits, its D2), the refusal style,
  the conformance layout under `conformance/reader/`, and, if decision D1 below goes to option A
  or C, the parser. What it does not share: the subset, the output, the oracle and the goal. It
  is related, not a part of it.
- `epic:tree-sitter-go-reader` (route C) measured what a syntax-only layer gives and recorded
  that function bodies need types; this epic reads the declared types of the annotations and
  leaves a body it cannot type opaque.
- `epic:java-quarkus-reader` is the model: `Entry` of kind `http` (`crates/model/src/lib.rs`)
  records the routes of a framework, `story:go-http-entries` does it for Go.
- `epic:release-fact-diff` is the consumer: `codespec diff` over the documents of two revisions.

## Decisions

### D1. Which parser reads TypeScript (open)

Options:

- (a) Strip the types, then read the ECMAScript with swell's parser. One parser for two
  languages; the reader stays small. But the types are what a TypeScript reader is for: the
  annotation of a handler's parameters, of an interface a route returns, of an imported function's
  result is the only type information a syntax-level reader has. Stripping them first throws the
  information away, and positions (`<file>:<line>`) move unless the stripper keeps them. swell's
  parser reads ECMAScript, not TypeScript; the stripper is a parser of TypeScript in itself.
- (b) A third-party TypeScript parser in Rust: candidates are `oxc_parser`, `swc_ecma_parser` and
  `tree-sitter-typescript`. It reads annotations, interfaces, generics and `import type`, with
  positions; the same pattern as `adapters/rust-read` (on `syn`) and as `epic:tree-sitter-go-reader`.
  The cost is one dependency whose release pace codespec does not set.
- (c) swell grows TypeScript. One parser for both, owned by the maintainers of swell. It is
  swell's work and swell's schedule, for a goal (service analysis) that is not swell's; it blocks
  this epic on another repository.

Recommended: (b). The reader needs the annotations, which (a) removes and (c) waits for; (b) is
the one option that starts without another repository and keeps the types. Which crate is the
first story's first step, measured: TypeScript syntax covered (a corpus of the constructs of the
slice), the position of each node as line and column, license, dependency size and build time
under `slot`, and a stable public API. The choice is reversible: the reader keeps every use of
the parser in one module, as `story:ecmascript-swell-surface` does for swell, so a change of
parser is the change of that module and the golden documents hold.

### D2. Types come from annotations and the file, not from `tsc`

The reader does not run the TypeScript compiler: no type checking, no inference across files, no
`node_modules` typings. A value whose type is not declared by an annotation or a literal in the
file is untyped, and what needs its type is refused. Option B (not taken): shell out to `tsc` for
the types, which makes the reader depend on Node and on the project's whole dependency tree, as
the Java reader depends on the project's jars; it may be a later epic, once this one shows
what the annotations alone give.

## Order

1. `story:typescript-node-service-reader`: the first slice.
2. Not drafted yet, named so that nobody reads the first story as the whole epic: effect
   summaries for Node's standard modules (`node:fs`, `node:http`, `node:child_process`) and the
   common HTTP clients, as `story:go-std-effect-summaries-common` does for Go; routes with
   middleware and `app.use` mounted across files; classes, `async` and `await`, `try`; the
   Vue single-file components of the same release (they are TypeScript inside a template format).
   Each is drafted when the first story's measurement on a real service shows it is the
   next-largest gap.

## Done When

- `story:typescript-node-service-reader` is implemented.
- `codespec diff` over two revisions of a TypeScript Node service gives the delta of an added
  route (`entry-added http GET /items`) and of an added function, and an `unknown-added` with its
  reason and `file:line` for every construct the slice refuses.

## Out of Scope

- Running TypeScript or writing it (a TypeScript writer), and the speed comparison of
  `epic:ecmascript-to-native`.
- The type checker of TypeScript (D2), JSX and TSX, Vue and Svelte templates, decorators.
- Any change to `cir/1` beyond what `story:typescript-node-service-reader` names (none expected:
  `Entry` of kind `http` exists).
