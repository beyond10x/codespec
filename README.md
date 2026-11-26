# codespec

<!-- badges: written by codespec-docs from website/data, do not edit -->
[![release](https://img.shields.io/github/v/release/beyond10x/codespec)](https://github.com/beyond10x/codespec/releases/latest)
[![check on main](https://img.shields.io/github/actions/workflow/status/beyond10x/codespec/check.yml?branch=main&label=check)](https://github.com/beyond10x/codespec/actions/workflows/check.yml?query=branch%3Amain)
[![licence](https://img.shields.io/github/license/beyond10x/codespec)](LICENSE)
[![languages read](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fbeyond10x%2Fcodespec%2Fmain%2Fwebsite%2Fdata%2Fbadges%2Flanguages-read.json)](https://beyond10x.github.io/codespec/docs/reference/support)
[![features verified](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fbeyond10x%2Fcodespec%2Fmain%2Fwebsite%2Fdata%2Fbadges%2Ffeatures-verified.json)](https://beyond10x.github.io/codespec/docs/reference/support)
[![definite or possible answers](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fbeyond10x%2Fcodespec%2Fmain%2Fwebsite%2Fdata%2Fbadges%2Fanswers-definite-possible.json)](https://beyond10x.github.io/codespec/docs/comparison#dim-s3)
<!-- slot: reader KPI badge of story:corpus-kpi-ratchet, added with its baseline -->
<!-- /badges -->

Code as typed data. codespec reads a source language into `cir/1`, a typed and canonical JSON
model of what code does inside a module. Passes rewrite it and compute facts about it, and writers emit
Go, Rust, C, Elixir, Gleam or Odin from it.

A harness runs the original and every output on the same seeded inputs and
compares what they print, byte for byte. Nothing in the pipeline is a language model: what a
reader cannot express is kept verbatim as a `foreign` statement and listed as a refusal.

**Documentation: <https://beyond10x.github.io/codespec/>** ·
[Getting started](https://beyond10x.github.io/codespec/docs/getting-started) ·
[Showcase](https://beyond10x.github.io/codespec/docs/examples/greeter) ·
[CLI reference](https://beyond10x.github.io/codespec/docs/reference/cli) ·
[cir/1 reference](https://beyond10x.github.io/codespec/docs/reference/cir-1) ·
[Status](https://beyond10x.github.io/codespec/docs/status) ·
[Comparison with other tools](https://beyond10x.github.io/codespec/docs/comparison)

```
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
```

## Why

Translating code between languages, or rewriting it within one, needs a representation of what a
function computes, with one definition and a check that does not depend on anyone's judgement.

In `cir/1` every expression carries its type, reading is strict, and a translation counts when the
original and the output produce the same bytes on the same generated inputs.
`codespec effects` reads the same model as a static analysis: per function, which effects it may
have, such as environment, file system, network and clock, from summaries of the library functions
it calls, and every call it cannot account for ([docs/static-analysis.md](docs/static-analysis.md)).

Read with every package it imports, a Go repository maps down to syscalls: `codespec effects`
lists the syscalls each function may reach, `codespec reach` the dead code from given entries,
`codespec trace` the call tree from one function down to its syscalls
([worked example](https://beyond10x.github.io/codespec/docs/examples/netcheck), `examples/netcheck`).
`codespec edit` changes Go and Rust source with a cq program: rename a function or a struct
field, point calls at another function, delete a function nothing calls. Each change is read
again and must be the expected `cir/1` before it is printed or written
([Code edits](https://beyond10x.github.io/codespec/docs/concepts/code-edits), `examples/edit`).
[Vision](https://beyond10x.github.io/codespec/docs/vision) has the rules and the milestones.

## Status

The support matrix lists the verified features of each reader and writer. [Support matrix](https://beyond10x.github.io/codespec/docs/reference/support).
For the changes in each release, see the [changelog](https://github.com/beyond10x/codespec/blob/main/CHANGELOG.md).

## Install

Download the archive for your platform from the
[GitHub Releases](https://github.com/beyond10x/codespec/releases), `codespec-<tag>-<target>.tar.gz`
for `x86_64-unknown-linux-musl`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` or
`aarch64-apple-darwin`, and check it against `SHA256SUMS`. On x86-64 Linux, pin the musl archive:
its binaries are static and run on any glibc and in `alpine`. The directory you extract it into is the `--adapters` directory. Keep the
`beam/` directory beside the Elixir and Gleam programs when copying them from a build or archive.

Or build from source, with Rust 1.97 (`rust-toolchain.toml`), Go 1.23 or newer and
[go-task](https://taskfile.dev). Building every adapter also needs the JDK, Groovy, Erlang/OTP, Elixir, Gleam and Odin. The native versions used in CI are in [.github/workflows/check.yml](.github/workflows/check.yml).

```console
git clone https://github.com/beyond10x/codespec && cd codespec
task build
```

## Five minutes with `orders`

`examples/orders` is a small Go order service. Read it, pass it, write it as Go and as Rust, and
compare all three:

```console
mkdir -p try
target/debug/codespec read --lang go examples/orders/source --out try/orders.read.cir.json --adapters target/adapters
target/debug/codespec pass try/orders.read.cir.json --out try/orders.cir.json
target/debug/codespec write --lang go try/orders.read.cir.json --out try/go --adapters target/adapters
target/debug/codespec write --lang rust try/orders.cir.json --out try/rust --adapters target/adapters
target/debug/codespec harness examples/orders --seeds 200 --commands 300 --build-dir target/harness
```

The last command prints
`harness: 200 seeds x 300 commands, 3 implementations (original, go, rust), 79990 output lines each, byte-equal`.
[Getting started](https://beyond10x.github.io/codespec/docs/getting-started) shows every step's
output.

## Layout

| path | what |
|---|---|
| `crates/model` | the definition of `cir/1` |
| `crates/passes` | `lookup-guard` and the facts |
| `crates/harness` | the comparison harness |
| `crates/cli` | the `codespec` command |
| `crates/edit` | the code edits behind `codespec edit` |
| `crates/docs` | `codespec-docs`, the generator of the site's derived pages |
| `adapters/go` | the Go reader and the Go writer |
| `adapters/rust` | the Rust writer |
| `adapters/elixir`, `adapters/gleam`, `adapters/odin` | readers, writers and native runners for typed scalar functions ([translation guide](website/docs/guides/elixir-gleam-odin.md)) |
| `schemas/cir-1.schema.json` | the JSON Schema, generated from the model |
| `examples/orders`, `examples/greeter` | Go programs, their `cir/1` documents and their Go and Rust outputs |
| `examples/edit` | a Go module and a Rust crate with one edit program per operation and refusal, and their expected diffs and refusals |
| `website/` | the documentation site |

Changing the repository: [AGENTS.md](AGENTS.md).

## License

Apache-2.0.
