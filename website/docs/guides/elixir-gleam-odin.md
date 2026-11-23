---
title: Translate Elixir, Gleam and Odin
sidebar_position: 5
description: Read typed scalar functions from Elixir, Gleam or Odin, write them in another language, and compare the native outputs.
---

# Translate Elixir, Gleam and Odin

The adapter names are `elixir`, `gleam` and `odin` for both `codespec read` and `codespec write`. Build them with `task build`. The build uses Elixir, Erlang/OTP, Gleam and Odin alongside the existing CodeSpec toolchain. CI pins the native tool versions in `.github/workflows/check.yml`. Gleam resolves the parser packages in its checked-in manifest on the first build.

The build puts shared BEAM bytecode in `target/adapters/beam/`. Keep `beam/` beside the executables when copying the adapters. The Gleam and Odin runners also need their respective compilers on `PATH`.

## Run a translation

The `stmt-if` conformance case returns a different integer for a positive, negative or zero input. This example reads the handwritten Elixir function and runs both its Gleam translation and its original on the same calls.

```sh
export CODESPEC_ADAPTERS="$PWD/target/adapters"
mkdir -p target/translate
case_dir=conformance/cases/stmt-if
target/debug/codespec read --lang elixir "$case_dir/elixir" --out target/translate/read.json
target/debug/codespec pass target/translate/read.json --out target/translate/passed.json
target/debug/codespec write --lang gleam target/translate/passed.json --out target/translate/gleam
"$CODESPEC_ADAPTERS/codespec-run-elixir" "$case_dir/elixir" "$case_dir/calls.json" > target/translate/original.txt
"$CODESPEC_ADAPTERS/codespec-run-gleam" target/translate/gleam "$case_dir/calls.json" > target/translate/written.txt
cmp target/translate/original.txt target/translate/written.txt
```

Use `--lang odin` to write an Odin package. The same case has handwritten inputs under `gleam/` and `odin/`. Run `task language-adapters-check` to compare each input with all three outputs.

## Supported source

- **Elixir** reads one `defmodule` from an `.ex` file or a directory containing one module. Give each function a scalar `@spec`. The reader parses quoted syntax without expanding macros. The writer emits `module.ex`.
- **Gleam** reads a `.gleam` file or a single-module source directory, including a package's `src/`. Annotate function parameters and results with `Int`, `Bool` or `String`. The reader uses the Glance parser. The writer emits a `gleam.toml` and a `src/` module for the Erlang backend. The runner uses the Gleam compiler on the source module.
- **Odin** reads scalar procedures from a source file or package directory using Odin's `core:odin/parser`. The writer emits `module.odin`. The runner compiles a driver against the package using `odin`.

The supported forms include local calls, bindings, comparisons and conditional returns. Native Elixir and Gleam permit arbitrarily large integers on the BEAM. Their readers retain arithmetic whose result may exceed the signed `cir/1` domain as foreign source. An explicit 64-bit bit-array normalization can express wrapping arithmetic. The handwritten `op-add` sources show the accepted form in each language.

A reader records unsupported function bodies or statements with their source and a refusal, or rejects an input whose declarations it cannot represent. A writer reports an unsupported construct before emitting its module. Each adapter publishes its feature claim through `--describe`. The [support matrix](../reference/support.mdx) links the claims to their native conformance runs.
