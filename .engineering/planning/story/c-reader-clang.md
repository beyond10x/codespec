---
format: aep.planning-md/3
id: story:c-reader-clang
kind: story
status: implemented
title: A C reader on clang's typed AST
relations:
- decomposes: epic:c-reader-writer
scope:
- confidence: cited
  path: adapters/c-read
- confidence: cited
  path: conformance/cases
- confidence: cited
  path: crates/conformance/src/conform.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-09T04:18:55Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
# Story: A C reader on clang's typed AST

## Outcome

`codespec-read-c <source dir>` (crate `adapters/c-read`) runs
`clang -Xclang -ast-dump=json -fsyntax-only -std=c11` on each `.c` file and prints one `cir/1`
module. It reads functions, `struct` types, `enum` types, fixed-width and plain integer types
(`int8_t`…`uint64_t`, `int`, `long`, `unsigned`), `_Bool`, string literals passed to
`printf`-family externs, local and global variables, `if`, `for`, `while`, `do`, `switch`, `break`,
`continue`, `return`, assignments, compound assignments, `++`/`--`, unary and binary operators,
calls, and struct field access. Every expression carries its type from clang's `qualType`. A call to
a function the module does not define is an extern with its declared signature, so
`codespec effects` links libc calls.

Anything outside that list is kept as a `foreign` statement or an opaque function, with a refusal
giving its position and reason: pointer arithmetic, unions, `goto`, variadic definitions, inline
assembly, function pointers.

## Acceptance

- Every kit case whose features the reader claims has a C source `c/case.c` under
  `conformance/cases/<case>/`. `codespec_conformance::conform::reader_corpus` builds a `c` corpus
  from it (`CORPUS_LANGS` gains `c`).
- A crate test reads `expr-binop`, `stmt-for` and one `switch` case with `codespec-read-c`, writes
  each with `codespec-write-go`, runs it with `codespec-run-go`, and gets the case's `expected.txt`.
  This needs no C runner.
- The reader does not claim `sem.int.wrap`. A test reads a C source with a signed `+` that overflows
  and asserts the feature is absent from the reader's `--describe` claims.
- A file using `goto` reads with one refusal at the `goto`'s line and exits 0.
- With `clang` missing on `PATH`, the reader exits 1 and prints `codespec-read-c: clang not found`.
- `task check` green. The reader adds no `REPORTS` entry, because `docs-check` re-runs every entry
  with the runner of its language and `codespec-run-c` comes from `story:c-writer`.
- The reader's `REPORTS` row, its `codespec conform` report and `reader-c.json` are the epic's
  Done When, done by the coordinator after both C stories merge.

## Out of Scope

- The writer and the runner (`story:c-writer`).
- Headers other than the C standard library's. A source including a project header is read only
  when the header is in the source dir.

## Shared surface and merge order

The operator asked (2026-11-09) for `story:c-reader-clang`, `story:c-writer`,
`story:builtin-defect-checks` and `story:use-cases-page` to run in one wave. They share these
files, each by appending one entry:

- `Cargo.toml` `[workspace] members` and `Cargo.lock`: `adapters/c-read`, `adapters/c`,
  `adapters/c-run`, `crates/check`.
- `.github/workflows/check.yml`: one test-matrix line per new crate.
- `Taskfile.yml`: the `adapters` task and the manifest loops (C stories only).
- `crates/docs/src/main.rs` `REPORTS` (C writer only), `crates/docs/src/support.rs` `LANGUAGES` and the generated
  `website/data/support-matrix.json` (C stories only).
- `crates/cli/src/lib.rs` `Cmd` (`story:builtin-defect-checks` only adds `check`; the C stories add
  no command).

Each implementor appends its own entries and touches no other story's. The coordinator merges in the
order `story:c-writer`, `story:c-reader-clang`, `story:builtin-defect-checks`,
`story:use-cases-page`, resolves the append conflicts in those files by keeping both entries, and
runs `task check` after each merge.

Generated files (`website/docs/reference/crates.md`, `website/docs/reference/cli.md`,
`website/data/support-matrix.json`, `website/data/conformance/*.json`) are never merged by hand:
the coordinator takes either side and runs `task docs-generate` after each merge.
`crates/docs/src/support.rs` `LANGUAGES` gains `c` in `story:c-writer` only. `story:c-reader-clang`
does not edit `LANGUAGES` or `REPORTS`.
