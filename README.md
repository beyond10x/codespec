# codespec

A typed code intermediate representation, `cir/1`, and the tools around it: readers that lift a
source language into it, passes that rewrite it, and writers that emit a source language from it.

```
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
```

Use cases:

- **Round trip with rewrites**: Go → `cir/1` → Go, with normalisation or idiom changes in between.
- **Translation**: Go → `cir/1` → Rust, and back.
- **One analysis for many languages**: sharing, fallibility, purity and iteration order computed on
  `cir/1` once.

Every translation is checked by running the original and the output on the same seeded inputs and
comparing the outputs byte for byte. Nothing in the pipeline is a language model: what a reader
cannot express is kept verbatim as a `foreign` statement and listed as a refusal.

Status: `cir/1` model and schema, the `codespec` CLI and a Go reader. Writers, passes and the
comparison harness follow ([docs/design.md](docs/design.md), § Milestones).

## Use

Needs Rust, Go and [go-task](https://taskfile.dev).

```console
task build
target/debug/codespec read --lang go examples/orders/source --out orders.cir.json --adapters target/adapters
target/debug/codespec validate --canonical orders.cir.json
target/debug/codespec schema
```

## Layout

| path | what |
|---|---|
| `crates/model` | the definition of `cir/1` |
| `crates/cli` | the `codespec` command |
| `schemas/cir-1.schema.json` | the JSON Schema, generated from the model |
| `adapters/go` | the Go reader (`go/types`) |
| `examples/orders` | a small Go service and its `cir/1` document |

## License

Apache-2.0.
