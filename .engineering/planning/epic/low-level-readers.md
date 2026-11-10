---
format: aep.planning-md/3
id: epic:low-level-readers
kind: epic
status: draft
title: Readers for LLVM IR and assembly files
summary: codespec reads one textual LLVM IR file or one x86-64 assembly file into a cir/1 module.
revision: 2
---
# Epic: Readers for LLVM IR and assembly files

## Outcome

`codespec read --lang llvm-ir <file.ll>` and `codespec read --lang asm <file.s>` each print one
`cir/1` module for one file, so code that exists only as compiler output can be passed, queried and
written like source read by the other readers.

## Why Now

Requested by the operator on 2026-11-10: "codespec file readers: llvm_ir, asm".

## Done When

- `story:reader-llvm-ir` and `story:reader-asm` are implemented.
- Both readers have a report `website/data/conformance/reader-<lang>.json` with `conforms: true`.

## Out of Scope

- Writers for LLVM IR or assembly. (Each reader story owns the runner the reader check needs at
  step 1.)
- LLVM bitcode (`.bc`), object files and disassembly of binaries.
- Assembly dialects other than the one `story:reader-asm` names.
