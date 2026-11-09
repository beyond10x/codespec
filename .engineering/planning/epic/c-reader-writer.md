---
format: aep.planning-md/3
id: epic:c-reader-writer
kind: epic
status: implemented
title: A C reader and a C writer
summary: codespec-read-c on clang's typed JSON AST, codespec-write-c with a header-only runtime, and codespec-run-c, verified by codespec conform.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T04:51:11Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-09T04:51:11Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-09T04:51:11Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
---
# Epic: A C reader and a C writer

## Outcome

`codespec read --lang c <source dir>` lifts C11 source into `cir/1` with a type on every
expression, and `codespec write --lang c` writes `cir/1` as C11 that builds with `cc` and runs.
Both are verified by `codespec conform`. The reader is `codespec-read-c`, the writer
`codespec-write-c`, the runner `codespec-run-c`.

## Why Now

Requested by the operator on 2026-11-09. The Go reader skips cgo files
(`adapters/go/cmd/codespec-read-go/front.go:57-60`), and `go/types` cannot type C, so C code reached
from Go is unread. A C reader closes that for analysis (calls, effects, the syscall barrier through
libc) and adds a fifth source language. A C writer gives a target with no runtime of its own.

## Decision

Each language is read with the tooling that knows its types (`docs/design.md`). For C that is
clang: `clang -Xclang -ast-dump=json -fsyntax-only` prints a typed AST, every expression with its
`qualType`. The adapter calls `clang` as a process and parses its JSON, so the workspace gains no
dependency. tree-sitter stays out (`docs/design.md` § 7: no types). The adapters are Rust crates in
the workspace, built into `target/adapters` by `task build`: `adapters/c-read` (`codespec-read-c`,
`story:c-reader-clang`), `adapters/c` (`codespec-write-c`) and `adapters/c-run` (`codespec-run-c`),
both `story:c-writer`.

## Scope

- `story:c-reader-clang`: the reader.
- `story:c-writer`: the writer and the runner `codespec-run-c`, which the reader's conformance run
  also needs.

## Out of Scope

- The preprocessor beyond what clang expands: macros are read as clang expands them, and their
  names are not kept.
- cgo: reading the `import "C"` files of a Go package together with their C.
- Pointer arithmetic, unions, `goto`, `setjmp`, variadic functions declared by the code, inline
  assembly: kept as `foreign` with a refusal.

## Risks

- Signed overflow is undefined behaviour in C, and `cir/1` integers wrap (`sem.int.wrap`). The
  writer must not emit a signed `+`, `-` or `*` whose wrap the program relies on. The reader must
  not claim `sem.int.wrap` for C source.
- `cir/1` strings, slices, maps and `(T, error)` results have no C counterpart. The writer needs a
  small runtime header, and the reader reads only what has a typed C form.

## Done When

- `story:c-writer` and `story:c-reader-clang` are implemented. `story:c-writer` adds `c` to
  `crates/docs/src/support.rs` `LANGUAGES` and the C writer row.
- After both merge, the coordinator adds the C reader to `crates/docs/src/main.rs` `REPORTS` and runs
  `codespec conform --lang c --role reader` (it needs `codespec-run-c` from the writer story and
  `codespec-read-c` from the reader story). It conforms on every feature the reader claims, and
  `task docs-generate` checks in `website/data/conformance/reader-c.json` and the C reader row of
  the support matrix.
- `task check` is green on the merge.
