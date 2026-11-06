# Changelog

Every change a user of `codespec` or of `cir/1` would notice.

## Unreleased

### Added

- `codespec features` prints the `cir/1` feature vocabulary (`codespec-features/1`, 105 ids), checked
  in as `schemas/cir-1.features.json`; `features_from_json` reads it strictly.
- `cir/1` intrinsic `sorted_keys`: the keys of a map in ascending order (integers numerically, strings
  by UTF-16 code unit). No writer renders it yet.
- `codespec-read-java` (`adapters/java`): reads a Java 17 package through javac into `cir/1`, refusing
  every form whose meaning the document would not keep. Needs a JDK (default path: JDK 21).
- `codespec-read-groovy` (`adapters/groovy`): reads a directory of Groovy 2.4 files into `cir/1`,
  typed by the Groovy static type checker whether a class is annotated or not. Dynamic code is kept
  as `foreign` with a refusal in one of eight categories (`adapters/groovy/README.md`). Needs a JDK
  and Groovy 2.4.10 (`GROOVY_HOME`, default: the sdkman `groovy/2.4.10` candidate).
- `codespec-read-rust` reads the Rust writer's loops, switches and multi-assigns back: the `series`
  output reads with 0 refusals.
- `examples/series` runs a fourth implementation, the Rust output read back and written as Go
  (`out/go-from-rust`): 4 implementations byte-equal under the harness.
- `examples/ledger`: a Java package read with `codespec-read-java`, written as Go and Rust, and run
  byte-equal with the original Java under the harness.
- The conformance kit's reference cases: `conformance/kit.json` and 100 cases, one subject feature
  each, checked by `codespec-conformance` (`task conformance-cases`).
- `codespec-write-go` writes `sorted_keys`, converts a `len` of a non-`int` type, upper-cases the
  first letter of an exported lower-case name, and renames a name that would clash (a field beside
  its method, a Go built-in, an import).
- `codespec-write-rust` writes `conv` to a built-in integer and `sorted_keys`, and refuses a
  function it cannot write as valid Rust instead of failing the whole module.
- `codespec-read-go` reads a labeled loop whose label only its own `break` and `continue` use as the
  plain loop.

## 0.2.0

### Added

- `cir/1` statements `for` (`init`, `cond`, `post`, each optional), `break` and `continue`. They are
  additive: every earlier document reads unchanged; a document using them is refused by a 0.1.2
  `codespec`.
- `codespec-read-go` reads three-clause and condition-only `for` loops, unlabeled `break` and
  `continue`, and `switch` with or without a tag. A `break` that would leave a `switch`, a labeled
  branch, `goto` and `fallthrough` are kept as `foreign` with a refusal.
- `codespec-write-go` writes them back; a `break` inside a `switch` gets a label on its loop.
- `codespec-write-rust` writes `for` as `while` or `loop` in a block holding `init`, runs `post`
  before each `continue`, writes `a, b = x, y` as every value bound first and then each target
  assigned, and moves a local slice or map that no later statement reads instead of refusing the
  copy.
- `examples/series`: an algorithmic Go package (insertion sort, sliding window, gcd, Collatz steps,
  trial-division primes, `switch` classification, integer overflow). Read, passed, written as Go and
  as Rust with no refusal; the harness trace `series` runs the original, the Go output and the Rust
  output on 200 seeds of 300 commands byte-equal (`task examples-series`).
- The documentation site has a generated page for `series`, `/docs/examples/series`: the Go
  source, the `cir/1` of four functions drawn as a tree, and each Go construct beside the Rust
  written from it.

## 0.1.2

### Fixed

- The release archives now hold `codespec-read-rust`; the `0.1.1` archives left it out. The release
  smoke test reads `examples/inventory` with it and compares the document with the checked-in one.

## 0.1.1

The first published binaries. The `0.1.0` tag exists, but its release build failed and it has no
release.

### Fixed

- `codespec-read-go` built with `-trimpath`, as the release archives are, finds the standard
  library through `go env GOROOT` instead of failing with `could not import errors`.
- The release build runs its macOS archive on `macos-15`; GitHub no longer offers `macos-14`.

## 0.1.0

The first release: the model, the Go and Rust readers, the passes, the Go and Rust writers and the harness.

### Added

- `cir/1`: the model in `b10x-codespec-model`, strict reading (unknown fields, unknown kinds, an
  untyped expression and any other format are refused), canonical JSON, and
  `schemas/cir-1.schema.json` generated from it.
- `codespec validate [--canonical]`, `codespec schema`, and `codespec read` / `codespec write`,
  which run a language's adapter and validate what crosses to and from it.
- `codespec-read-go`: a Go reader on `go/types`. A construct `cir/1` cannot express is kept as a
  `foreign` statement and listed as a refusal. Its `reader.version` is 0.1.0.
- `codespec-read-rust`: a Rust reader on `syn` with its own name and type resolution for the
  `cir/1` subset (`codespec read --lang rust <crate dir>`). It maps Rust's idioms back to the
  Go-shaped forms the passes read: `Result<T, Error>` to `(T, error)`, `Option<T>` to `(T, bool)`,
  `&mut self` to pointer receivers, `enum Error` to sentinel errors, `Display` to `String()` and
  `Error()`. A construct it cannot type is kept as a `foreign` statement with a refusal at its line.
  The Rust writer's `orders` output reads back to the passed document but for three named
  exceptions.
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
- `examples/inventory`: a hand-written Rust crate read, passed and written as Go and as Rust; the
  original, the Go output and the Rust output agree on 200 traces of 300 commands.
- `harness.toml` `trace`: `orders` (the default) or `inventory` selects the trace generator.
- The documentation site, <https://beyond10x.github.io/codespec/>, with its generated CLI, crate,
  `cir/1` and status pages and the showcase, from `codespec-docs`.
