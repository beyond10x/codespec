---
title: codesec
sidebar_position: 5
description: codesec is a private companion to codespec, a library of defect patterns run over cir/1 and the review work done with it. What it looks for, how it works and what stays private.
lede: The engine is public and documented on this site. The pattern library and its results are not.
source: crates/check/src/lib.rs, crates/effects/src/lib.rs, crates/cli/src/lib.rs
---

# codesec

codesec is a private companion to codespec. It is a library of defect patterns, written as queries
over `cir/1` and run over code a codespec reader has read, and the review work done with it. The
engine is public and documented on this site: the readers, `cir/1`, `codespec check`,
`codespec reach` and `codespec effects`. The pattern library and its results are not.

## What it looks for

Classes of defect, each resting on something the public engine already shows:

| class | what the public engine shows |
|---|---|
| a dropped error | the `dropped-error` check of [`codespec check`](/docs/reference/cli#codespec-check) |
| a lock without an unlock on every path | the `lock-without-unlock` check of [`codespec check`](/docs/reference/cli#codespec-check) |
| code an entry point reaches through an unresolved call | [`codespec reach`](/docs/concepts/static-analysis#entries-and-dead-code-codespec-reach) and the [`unknown` entries](/docs/concepts/static-analysis#what-unknown-means) of `codespec effects` |
| a call that reaches the file system, the network or a process | the effects `fs.read`, `fs.write`, `net` and `proc` of [`codespec effects`](/docs/concepts/static-analysis#the-effect-vocabulary) |

## How it works

1. Read the code with a codespec reader. What the reader cannot express is listed as a refusal;
   nothing is guessed.
2. Run the patterns as queries over the `cir/1` documents. Each match is a candidate.
3. Review each candidate by hand in the source. A candidate counts as a finding only when a
   reproducer test confirms it.

## What stays private, and why

- the exact form of the patterns
- which code was reviewed
- every finding, until the maintainers of that code have fixed it

A finding goes to the maintainers of the code first. The name of the reviewed code or the exact
pattern would point others to a defect before its fix is out.

## What you can do with the public tool

- [Map a repository to syscalls](/docs/guides/map-a-repository-to-syscalls): read a repository with
  every package it imports and list the effects and syscalls each function may reach.
- [Static analysis](/docs/concepts/static-analysis): effects, dispatch, entry points and what
  `unknown` means.
- [`codespec check`](/docs/reference/cli#codespec-check): the built-in defect checks, one finding
  per line.

## Contact

To ask about a review, [open an issue](https://github.com/beyond10x/codespec/issues) on the codespec
repository.
