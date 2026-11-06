# codespec

Code as typed data. codespec reads a source language into `cir/1`, a typed and canonical JSON
model of what code does inside a module; passes rewrite it and compute facts about it; writers emit
Go or Rust from it. A harness runs the original and every output on the same seeded inputs and
compares what they print, byte for byte. Nothing in the pipeline is a language model: what a
reader cannot express is kept verbatim as a `foreign` statement and listed as a refusal.

**Documentation: <https://beyond10x.github.io/codespec/>** ·
[Getting started](https://beyond10x.github.io/codespec/docs/getting-started) ·
[Showcase](https://beyond10x.github.io/codespec/docs/examples/greeter) ·
[CLI reference](https://beyond10x.github.io/codespec/docs/reference/cli) ·
[cir/1 reference](https://beyond10x.github.io/codespec/docs/reference/cir-1) ·
[Status](https://beyond10x.github.io/codespec/docs/status)

```
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
```

## Why

Translating code between languages, or rewriting it within one, needs a representation of what a
function computes, with one definition and a check that does not depend on anyone's judgement. In
`cir/1` every expression carries its type, reading is strict, and a translation counts when the
original and the output produce the same bytes on the same generated inputs.
[Vision](https://beyond10x.github.io/codespec/docs/vision) has the rules and the milestones.

## Status

**0.2.0**: the `cir/1` model and schema, the `codespec` CLI, a Go reader, the passes and facts, a Go
writer, a Rust writer, a Rust reader (Rust → `cir/1` → Go, on `examples/inventory`) and the comparison
harness, shown on four examples, among them the algorithmic `examples/series`. Every
capability and the test that holds it: [Status](https://beyond10x.github.io/codespec/docs/status). Changes:
[CHANGELOG.md](CHANGELOG.md).

## Install

Download the archive for your platform from the
[GitHub Releases](https://github.com/beyond10x/codespec/releases), `codespec-<tag>-<target>.tar.gz`
for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` or `aarch64-apple-darwin`, and check it
against `SHA256SUMS`. It holds `codespec` and the adapters `codespec-read-go`, `codespec-read-rust`, `codespec-write-go`
and `codespec-write-rust`; the directory you extract it into is the `--adapters` directory.

Or build from source, with Rust 1.97 (`rust-toolchain.toml`), Go 1.23 or newer and
[go-task](https://taskfile.dev):

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
| `crates/docs` | `codespec-docs`, the generator of the site's derived pages |
| `adapters/go` | the Go reader and the Go writer |
| `adapters/rust` | the Rust writer |
| `schemas/cir-1.schema.json` | the JSON Schema, generated from the model |
| `examples/orders`, `examples/greeter` | Go programs, their `cir/1` documents and their Go and Rust outputs |
| `website/` | the documentation site |

Changing the repository: [AGENTS.md](AGENTS.md).

## License

Apache-2.0.
