---
title: Add a reader or writer adapter
sidebar_position: 2
description: The program contract a reader or writer adapter keeps, where it lives, how codespec finds it, and the checks it must pass.
lede: An adapter is a program named codespec-read-LANG or codespec-write-LANG; codespec finds it by name and refuses anything it prints that the model does not accept.
source: crates/cli/src/main.rs, Taskfile.yml, adapters/go/cmd, adapters/rust; every command run in this repository on 2026-11-06
---

# Add a reader or writer adapter

## The contract

| | Reader | Writer |
|---|---|---|
| program name | `codespec-read-<lang>` | `codespec-write-<lang>` |
| arguments | one: the source (for Go, a package directory) | one: the output directory |
| stdin | — | one `cir/1` module, validated by `codespec write` |
| stdout | one `cir/1` module, validated by `codespec read` | free |
| success | exit 0 | exit 0; a function it cannot write is left out and listed, if the target has a refusal list |
| failure | non-zero exit, a message on stderr | non-zero exit, a message on stderr, nothing written |

`codespec` looks the program up in the `--adapters` directory (or `CODESPEC_ADAPTERS`); without
it, the command fails before anything runs:

```shell-session
$ ls target/adapters
codespec-read-go
codespec-write-go
codespec-write-rust
$ target/debug/codespec read --lang java examples/orders/source --out try/x.json --adapters target/adapters
codespec: no read adapter for "java": target/adapters/codespec-read-java does not exist
```

A reader must be deterministic: the same source read twice gives the same bytes. A construct it
cannot express becomes a `foreign` statement with a `refusals` entry, never an approximation. A
writer reads its choices off `Module.facts` and refuses what they do not decide
([Readers, passes and writers](/docs/concepts/pipeline)).

## Where it lives and what it is written in

An adapter goes under `adapters/<language>/`, written in the language whose tooling reads or
writes its target best: the Go adapters are a Go module (`adapters/go/cmd/codespec-read-go`,
`adapters/go/cmd/codespec-write-go`), the Rust writer is the workspace package
`codespec-write-rust` in `adapters/rust`. tree-sitter is not used: it gives syntax only and `cir/1`
needs a type on every expression ([Not used](/docs/vision#not-used)). The repository's own tooling
(the model, the CLI, passes, harness and checks) stays Rust.

Build it into `target/adapters` from the `adapters` task in `Taskfile.yml`, next to the existing
ones, so `task build` produces it.

## The boundary is checked

A stand-in reader that only copies a document shows what `codespec read` does with an adapter's
output. This is a throwaway shell script in a scratch directory, not an adapter to keep:

```shell-session
$ cat work/stub/codespec-read-cat
#!/bin/sh
exec cat "$1"
$ target/debug/codespec read --lang cat examples/orders/orders.read.cir.json --out work/copy.json --adapters work/stub
$ cmp work/copy.json examples/orders/orders.read.cir.json
```

A document the model accepts goes through, re-encoded canonically. One with a field the model does
not have is refused, and no file is written:

```shell-session
$ jq -c '. + {extra: 1}' examples/orders/orders.read.cir.json > work/broken.json
$ target/debug/codespec read --lang cat work/broken.json --out work/copy2.json --adapters work/stub
codespec: work/stub/codespec-read-cat: not a cir/1 module: unknown field `extra`, expected one of `format`, `name`, `reader`, `doc`, `types`, `consts`, `errors`, `funcs`, `layout`, `facts`, `refusals`
$ test -e work/copy2.json || echo 'nothing written'
nothing written
```

Fix the adapter, never the model's strictness.

## What the gate needs

1. Format and lint in the adapter's own language, in the `adapters-check` task (`gofmt` and
   `go vet` for Go; `cargo fmt` and `cargo clippy` cover a Rust adapter in the workspace).
2. A reader: every example read twice to the same bytes, equal to the checked-in document, in
   `examples-check`.
3. A writer: its output for every example checked in under `examples/<name>/out/<language>/`,
   compared in the gate, and built or compiled by the target's own toolchain without warnings
   (`examples-go`, `examples-rust`, `examples-greeter`).
4. For a translation to count, an implementation entry in the example's `harness.toml`, so
   `codespec harness` compares its output byte for byte with the original's.

Run `task check`; it is the gate.
