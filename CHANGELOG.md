# Changelog

Every change a user of `codespec` or of `cir/1` would notice. Nothing is released yet.

## Unreleased

### Added

- `cir/1`: the model in `b10x-codespec-model`, strict reading (unknown fields, unknown kinds, an
  untyped expression and any other format are refused), canonical JSON, and
  `schemas/cir-1.schema.json` generated from it.
- `codespec validate [--canonical]`, `codespec schema`, and `codespec read` / `codespec write`,
  which run a language's adapter and validate what crosses to and from it.
- `codespec-read-go`: a Go reader on `go/types`. A construct `cir/1` cannot express is kept as a
  `foreign` statement and listed as a refusal.
- `examples/orders`: a Go order service and its `cir/1` document.
