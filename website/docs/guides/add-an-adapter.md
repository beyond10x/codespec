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

`codespec` looks the program up in the `--adapters` directory (or `CODESPEC_ADAPTERS`). Without
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
cannot express becomes a `foreign` statement with a `refusals` entry. It is not approximated. A
writer reads its choices off `Module.facts` and refuses what they do not decide
([Readers, passes and writers](/docs/concepts/pipeline)).

## Where it lives and what it is written in

An adapter goes under `adapters/<language>/`, written in the language whose tooling reads or
writes its target best. The Go adapters are a Go module (`adapters/go/cmd/codespec-read-go`,
`adapters/go/cmd/codespec-write-go`). The Rust writer is the workspace package
`codespec-write-rust` in `adapters/rust`. tree-sitter is not used: it gives syntax only, and
`cir/1` needs a type on every expression ([Not used](/docs/vision#not-used)). The repository's own
tooling stays Rust: the model, the CLI, passes, harness and checks.

Build it into `target/adapters` from the `adapters` task in `Taskfile.yml`, next to the existing
ones, so `task build` produces it.

## The boundary is checked

A stand-in reader that only copies a document shows what `codespec read` does with an adapter's
output. The script is a throwaway in a scratch directory, not an adapter to keep:

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

Fix the adapter. The model's strictness does not change.

## What the gate needs

1. Format and lint in the adapter's own language, in the `adapters-check` task: `gofmt` and
   `go vet` for Go. `cargo fmt` and `cargo clippy` cover a Rust adapter in the workspace.
2. A reader: every example read twice to the same bytes, equal to the checked-in document, in
   `examples-check`.
3. A writer: its output for every example checked in under `examples/<name>/out/<language>/`,
   compared in the gate, and built or compiled by the target's own toolchain without warnings
   (`examples-go`, `examples-rust`, `examples-greeter`).
4. For a translation to count, an implementation entry in the example's `harness.toml`, so
   `codespec harness` compares its output byte for byte with the original's.
5. A conformance report: `codespec conform` runs the conformance kit against the adapter and exits
   0 only when it conforms.

`codespec conform` needs two programs in the `--adapters` directory (or `CODESPEC_ADAPTERS`):

- the adapter, which answers `--describe` with its manifest, the features it claims;
- `codespec-run-<lang>`, the runner that builds what a writer wrote, or the source a reader reads,
  and runs it on a case's `calls.json`.

A reader also needs `codespec-write-go` and `codespec-run-go`, which write its output as Go and run
it, and `codespec-write-cir` and `codespec-run-cir`, the evaluator's pair, which judge a case the Go
writer refuses (such as a tagged union); they are looked up only when a case needs them. Each kit
case is then run, and every feature gets a verdict against the claim.

Here is the Go writer, run in this repository. `codespec-docs generate` writes this block from the
checked-in report `website/data/conformance/writer-go.json`, and `task docs-check` fails when it
differs:

<!-- conform-sample: written by codespec-docs from website/data/conformance/writer-go.json -->
```shell-session
$ export CODESPEC_ADAPTERS=target/adapters
$ target/debug/codespec conform --lang go --role writer --out work/go-writer.json
writer go 0.6.1: conforms; 126 claimed, 126 pass, 0 fail, 2 refused unclaimed, 0 pass unclaimed, 0 untested, 0 boundary failing
work/go-writer.json: written
$ jq -c .summary work/go-writer.json
{"claimed":126,"pass":126,"fail":0,"refused_unclaimed":2,"pass_unclaimed":0,"untested":0,"boundary_fail":0}
```
<!-- /conform-sample -->

The kit defaults to `conformance/` in this repository. Outside it, take
`codespec-conformance-<version>.tar.gz` from the release, unpack it, and pass its `conformance/`
directory as `--kit`.

Unpack a release's binary archive ([what it holds](/docs/getting-started#install)) into one
directory, put your own adapter and runner beside its programs, and pass the directory as
`--adapters`. That directory is everything `codespec conform` needs for a writer whose runner
it holds and for a reader. The runners build what they run, so `go`, `cargo` and, for C, `cc` must
be on `PATH`.

| flag | effect |
|---|---|
| `--corpus` | a reader's corpus; default `<kit>/reader/<lang>` |
| `--timeout` | bounds each program, in seconds; default 300 |
| `--check REPORT` | runs again and lists every feature whose entry differs from the report |

Run `task check`, the gate.
