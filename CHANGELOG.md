# Changelog

Every change a user of `codespec` or of `cir/1` would notice.

## 0.1.0

The first release: the model, the Go reader, the passes, the Go and Rust writers and the harness.

### Added

- `cir/1`: the model in `b10x-codespec-model`, strict reading (unknown fields, unknown kinds, an
  untyped expression and any other format are refused), canonical JSON, and
  `schemas/cir-1.schema.json` generated from it.
- `codespec validate [--canonical]`, `codespec schema`, and `codespec read` / `codespec write`,
  which run a language's adapter and validate what crosses to and from it.
- `codespec-read-go`: a Go reader on `go/types`. A construct `cir/1` cannot express is kept as a
  `foreign` statement and listed as a refusal. Its `reader.version` is 0.1.0.
- `codespec pass`: the `lookup-guard` rewrite (a comma-ok map lookup and its early-return guard
  become one `lookup_else`) and seven facts in `Module.facts` (`shared`, `messages`, `fallible`,
  `comma_ok`, `read_only`, `displays`, `unordered_iteration`). Running it on its own output gives
  the same bytes.
- `codespec-write-go`: a Go writer that builds `go/ast` and prints it with `go/printer`; the output
  is gofmt-clean and vetted, `foreign` Go is spliced back verbatim, and a construct Go cannot hold
  is refused with its position, nothing written.
- `codespec-write-rust`: a Rust writer on `syn` and `prettyplease` that writes a Cargo crate
  (`Cargo.toml`, `src/lib.rs`, `refusals.json`) from a module and its facts. A function it cannot
  write is left out and listed in `refusals.json`; a module without facts is refused whole. The
  crate builds without warnings.
- `codespec harness <example>`: builds every implementation an example's `harness.toml` lists, runs
  each on seeded traces (`--seeds`, `--commands`, `--build-dir`) and requires byte-equal output; a
  difference exits 1 naming the seed, the implementation and the first differing line.
- `examples/orders`: a Go order service, its `cir/1` documents, its Go and Rust outputs and the
  harness drivers. The original, the Go output and the Rust output agree on 200 traces of 300
  commands.
- `examples/greeter`: a Go greeting service written as Go and as Rust with no refusal.
- The documentation site, <https://beyond10x.github.io/codespec/>, with its generated CLI, crate,
  `cir/1` and status pages and the showcase, from `codespec-docs`.
