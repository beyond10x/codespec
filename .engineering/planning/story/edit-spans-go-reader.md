---
format: aep.planning-md/3
id: story:edit-spans-go-reader
kind: story
status: proposed
title: The Go reader gives spans and reads through an overlay
summary: decl, node and name spans from the Go reader, and --overlay for re-reading edited files
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:48Z", actor: "human:timo", revision: 3}
---
# Story: The Go reader gives spans and reads through an overlay

## Outcome

`codespec-read-go --spans <dir>` prints a spanned module (`docs/code-edit-language.md` § 6.3):
`decl` spans for every function and type (from the doc comment, else the declaration, to its end),
`node` spans for every statement, `name` spans for every function, struct field, `call`, `method`,
`extern`, `extern_method`, `func_ref`, `field` and keyed `struct` literal field, with `text` the
identifier as written; an `unspanned` entry where there is none (`implicit`: a positional literal
field, the hidden hop of a promoted field; `lowered: <why>`: a node the reader made, such as a hoisted
init's renamed local); `generated` on a file whose first comment is Go's `// Code generated …
DO NOT EDIT.` line. Offsets come from `token.Pos` through the file set.

`codespec-read-go --overlay <file> <dir>` reads with the overlay's contents in place of the named
files, both for the files it parses and for the files the source importer opens (`go/build`'s
`OpenFile` and `ReadDir` hooks), so a package importing an edited package type-checks against the
edit. `--describe` marks the reader as giving spans.

## Done When

- `adapters/go/cmd/codespec-read-go/spans_test.go`: for `examples/series` and `examples/orders`,
  `codespec read --spans` passes strict reading (`story:edit-spans-format`), and the `cir/1` is
  byte-identical to the checked-in `*.read.cir.json`.
- Named cases with expected span documents checked in under `testdata/spans/`: two functions on one
  line; a method with a doc comment; a file with CRLF line ends; a file with a byte-order mark; a
  multi-byte identifier; a positional and a keyed struct literal; a promoted field access; a
  hoisted `if` init (its statement spanned, its renamed local `lowered`); a generated file.
- An overlay case: package `b` renames `F` to `G` in the overlay only and package `a` calls `b.G`;
  reading `a` with the overlay gives an `extern` naming `G` and no refusal, without it a refusal.
- `gofmt -l` empty and `go vet` clean (`task adapters-check`). No test over 5 s.

## Scope

- Own: `adapters/go/cmd/codespec-read-go/` (`main.go` flags, `ir.go` span members, `encode.go`,
  `front.go` and `calls.go` where nodes are built, `spans.go` and `spans_test.go` new,
  `testdata/spans/`), `adapters/go/cmd/codespec-read-go/README.md`.
- Lands on the same files as the branch `wave/go-reader-refusals-1` (`front.go`, heavily) and the
  stories of `epic:go-reader-refusals`: starts after that branch is merged, and after any wave-2
  story of that epic that is active then.
