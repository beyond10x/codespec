---
format: aep.planning-md/3
id: story:edit-spans-go-reader
kind: story
status: proposed
title: The Go reader gives spans
summary: decl and name spans from the Go reader; held until wave/go-reader-refusals-1 is merged
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-spans-format
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:48Z", actor: "human:timo", revision: 3}
---
# Story: The Go reader gives spans

## Outcome

`codespec-read-go --spans <dir>` prints the object of `docs/code-edit-language.md` § 6.3: the
`cir/1` module unchanged, and `files`, `spans`, `unspanned`, `refs`. Spans are recorded on the
reader's own IR (`ir.go`) where nodes are built and their `at` paths computed while encoding:

- a `decl` span for every function (from its doc comment, else its declaration, to its closing
  brace);
- a `name` span for every function, struct field, `call`, `method`, `extern`, `extern_method`,
  `func_ref`, `field` and keyed `struct` literal field, `text` the identifier as written;
- an `unspanned` entry where there is none: `implicit` (a positional literal field, the hidden hop
  of a promoted field), `lowered: <why>` (a node the reader made);
- `generated` on a file whose first comment is Go's `// Code generated … DO NOT EDIT.` line.

Offsets come from `token.Pos` through the file set. No overlay flag: verification re-reads a cache
copy (§ 7.3).

## Hold

Starts only after the pull request of `wave/go-reader-refusals-1` is merged to `main` and the
integration branch `wave/code-edit-language` has merged `origin/main`. That branch rewrites
`front.go` and adds the lowered forms the named cases below cover.

## Done When

- `adapters/go/cmd/codespec-read-go/spans_test.go`: for `examples/series` and `examples/orders`,
  `codespec read --spans` passes strict reading (`story:edit-spans-format`) and the `cir/1` is
  byte-identical to the checked-in `*.read.cir.json`; the output object carries `files`.
- Named cases with expected span documents under `testdata/spans/`, each checked for its `name`
  spans and `unspanned` entries:
  - two functions on one line; a method with a doc comment; CRLF line ends; a byte-order mark; a
    multi-byte identifier; a positional and a keyed struct literal; a promoted field access; a
    generated file;
  - the forms of `epic:go-reader-refusals` wave 1: a blank assignment `_ = load()` (the call's
    `name` span); a local `var` with a call initializer and a local `const` (the call spanned, the
    omitted `const` giving no node); small literal forms (`&T{k: v}` keys, `new(T)`, an empty
    literal of another package's struct); named results read as locals (the made statement has
    no `name`-role node, the function's `decl` covers the whole); a hoisted `if` init and a
    hoisted `switch` init (`if v, err := load(); err != nil`: the call `load` keeps its span inside
    the hoisted statement; the renamed colliding local is not a wave-1 kind).
- `gofmt -l` empty and `go vet` clean (`task adapters-check`). No test over 5 s.

## Critic findings answered

- Overlay through `go/build` hooks breaks module resolution, `ParseDir` reads the disk (design,
  blocker): no overlay in the reader; `codespec edit` re-reads a copy of the module root, so the
  reader resolves imports as it does today.
- Waits on the unmerged `wave/go-reader-refusals-1` (parallel safety, blocker): the hold above,
  also in the epic's wave section and the design § 13; named cases cover that branch's new
  lowered forms.
- `--describe` spans mark untested (acceptance): no mark; the output object is tested.

## Scope

- Own: `adapters/go/cmd/codespec-read-go/` (`main.go` flag, `ir.go` span fields, `encode.go`
  paths, `front.go` and `calls.go` where nodes are built, `spans.go` and `spans_test.go` new,
  `testdata/spans/`, `README.md`).
- The same files as `wave/go-reader-refusals-1` and the later waves of `epic:go-reader-refusals`;
  hence the hold.
