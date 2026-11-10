---
format: aep.planning-md/3
id: story:reader-asm
kind: story
status: draft
title: A reader for assembly files
summary: codespec-read-asm reads one x86-64 GNU-syntax .s file into cir/1.
relations:
- decomposes: epic:low-level-readers
- depends_on: story:reader-llvm-ir
revision: 2
---
# Story: A reader for assembly files

## Outcome

`codespec-read-asm`, a Rust crate `adapters/asm-read`, reads one x86-64 assembly file in GNU
(AT&T) syntax, as `cc -S -O0` prints it on Linux, into one `cir/1` module: one function per global
function symbol, System V argument and return registers as parameters and result, stack slots as
variables, calls, arithmetic and comparisons, and control flow rebuilt from labels and jumps into
`if` and loops. What it cannot map stays a `foreign` statement with one refusal.

A register gives a width and no sign. The reader takes the sign from the instructions that use
a value (`idiv`, `jl`, `movs` against `div`, `jb`, `movz`) and refuses a function with a value no
such instruction fixes. The corpus carries each case's C prototypes beside the `.s` file for the
runner.

## Acceptance

- `codespec read --lang asm` on a checked-in `.s` fixture prints a valid `cir/1` module, the same
  bytes on a second read, equal to a checked-in expected document.
- The corpus `conformance/reader/asm` is generated from the kit's `c/case.c` sources by the pinned
  C compiler with `-S -O0`; `website/data/conformance/reader-asm.json` is generated and
  drift-checked like the other reader reports.
- That report has `conforms: true` and `summary.pass` of at least 25; every kit feature with a
  `c/case.c` that is not claimed is named in the crate doc with the reason.
- An instruction outside the reader's table is kept `foreign` with a refusal naming the mnemonic
  and the line; a reader test holds it.
- Any other architecture or syntax (Intel syntax, AArch64) is refused at the first line that shows
  it, with exit 1 and a message naming what was seen.
- `codespec-run-asm` assembles a corpus case with the pinned compiler, calls it through the
  prototypes file and prints the case's `expected.txt`.
- `--describe` prints a manifest that `task adapters` reads strictly.

## Scope

- Own: `adapters/asm-read` (new), `crates/cli` (the `--lang` value), `crates/conformance`
  (`CORPUS_LANGS`, the corpus builder), `crates/docs`, `Taskfile.yml`, the release workflow's
  program list, `AGENTS.md`, `CHANGELOG.md`, one site page. Inferred.
- Shares `crates/conformance`, `crates/cli`, `crates/docs` and `Taskfile.yml` with
  `story:reader-llvm-ir`: the two run one after the other.
- Inferred by the coordinator, not said by the operator: x86-64, GNU syntax, Linux.
