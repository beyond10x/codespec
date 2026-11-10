---
format: aep.planning-md/3
id: story:reader-llvm-ir
kind: story
status: draft
title: A reader for LLVM IR files
summary: codespec-read-llvm-ir reads one .ll file into cir/1, with control flow rebuilt from basic blocks.
relations:
- decomposes: epic:low-level-readers
revision: 2
---
# Story: A reader for LLVM IR files

## Outcome

`codespec-read-llvm-ir`, a Rust crate `adapters/llvm-ir-read`, reads one textual LLVM IR file
(`.ll`, as `clang -S -emit-llvm -O0` prints it) into one `cir/1` module: functions, integer and
floating types, structs, arrays, pointers, globals, calls, and control flow rebuilt from the basic
blocks into `if`, loops and `switch`. `alloca`, `load` and `store` of a local become a variable.
What it cannot map stays a `foreign` statement with one refusal, as `AGENTS.md` requires.

LLVM integer types carry no sign and `cir/1` requires one. The reader takes it from debug
information (`DIBasicType` encodings; the corpus is built with `-g`) and refuses a function whose
values have none. It does not infer a sign from instructions.

## Acceptance

- `codespec read --lang llvm-ir` on a checked-in `.ll` fixture prints a valid `cir/1` module, the
  same bytes on a second read, equal to a checked-in expected document.
- The corpus `conformance/reader/llvm-ir` is generated from the kit's `c/case.c` sources by
  `clang -S -emit-llvm -O0`, with the clang version pinned and a missing or different clang a
  named failure of the step; `website/data/conformance/reader-llvm-ir.json` is generated and
  drift-checked like the other reader reports.
- That report has `conforms: true` and `summary.pass` of at least 40; every kit feature with a
  `c/case.c` that is not claimed is named in the crate doc with the reason.
- A function whose blocks do not reduce to structured control flow is kept `foreign` with a
  refusal naming the function and the block; a reader test holds it on an irreducible fixture.
- `codespec-run-llvm-ir` (same crate or `adapters/llvm-ir-run`) runs a corpus case with the pinned
  clang and a calls driver, as `adapters/c-run` does, and prints the case's `expected.txt`.
- A `.ll` file without debug information is read with every function refused, exit 0, one
  refusal each naming the missing type information; a test holds it.
- `--describe` prints a manifest that `task adapters` reads strictly.

## Scope

- Own: `adapters/llvm-ir-read` (new), `crates/cli` (the `--lang` value), `crates/conformance`
  (`CORPUS_LANGS`, the corpus builder), `crates/docs`, `Taskfile.yml`, the release workflow's
  program list, `AGENTS.md`, `CHANGELOG.md`, one site page. Inferred.
- The reader parses the text itself: no dependency on an LLVM library.
