---
title: Readers, passes and writers
sidebar_position: 2
description: The adapter boundary, the passes and the seven facts, the Go and Rust writers, and how refusals travel through the pipeline.
lede: Readers and writers are separate programs that codespec checks at every crossing; passes in between compute the facts, so writers decide nothing themselves.
source: crates/cli/src/main.rs, crates/passes/src/lib.rs, crates/passes/tests/passes.rs, adapters/go/cmd, adapters/rust/src/write.rs, adapters/rust/tests/writer.rs
---

# Readers, passes and writers

```text
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
```

## Adapters

A reader and a writer are **adapters**: programs found by name in the `--adapters` directory (or
`CODESPEC_ADAPTERS`).

| Role | Program | Contract |
|---|---|---|
| reader | `codespec-read-<lang> <source>` | prints one `cir/1` module on stdout; exit 0 |
| writer | `codespec-write-<lang> <out dir>` | reads one `cir/1` module on stdin and writes into the directory; exit 0 |

`codespec read` validates the reader's output against the model before it writes a byte, and
writes it in canonical encoding. `codespec write` validates the document before the writer sees it.
No adapter's output reaches another unchecked. An adapter is written in the language whose tooling
handles its target best:

| Adapter | Language | Built on | Status |
|---|---|---|---|
| `codespec-read-go` | Go | `go/parser`, `go/types` | shipped |
| `codespec-write-go` | Go | `go/ast`, `go/printer`, `go/format` | shipped |
| `codespec-write-rust` | Rust | `syn`, `prettyplease` | shipped |
| `codespec-read-rust` | Rust | `syn`, with its own name and type resolution | shipped |
| `codespec-read-c` | Rust | the JSON AST `clang -Xclang -ast-dump=json` prints, typed by clang | shipped |
| `codespec-write-c` | Rust | C11 with the header-only runtime `codespec_rt.h` | shipped |
| `codespec-write-cir` | Rust | a copy of the module, which `codespec-run-cir` runs with the `cir/1` evaluator | shipped |
| `codespec-read-java` | Java | javac's attributed trees | shipped |
| `codespec-read-groovy` | Java | the Groovy 2.4 compiler and its static type checker | shipped |

How to add one: [Add a reader or writer adapter](/docs/guides/add-an-adapter).

## Passes and facts

`codespec pass` applies every pass to a module: first `lookup-guard` on every function body, then
the facts over the rewritten module. A pass is deterministic and total: it rewrites the pattern it
recognises, leaves every other node as it was, and never fails. Running `codespec pass` on its own
output gives the same bytes.

### `lookup-guard`

`v, ok := m[k]` directly followed by `if !ok { … return }`, with `ok` read
nowhere else and `v` not read in the guard, becomes one `lookup_else { name: v, map: m, index: k,
then }` statement. Rust writes it as `let Some(v) = … else { … };`. A `foreign` statement after the
lookup may read `ok`, so the lookup is then left as it was.

### Facts

The facts are written into `Module.facts` and read by writers:

| Fact | What it lists |
|---|---|
| `shared` | struct types `T` such that `*T` is stored in a struct or union field, or as the element or key of a map, slice or channel |
| `messages` | declared types of values sent on a channel of `any` |
| `fallible` | functions whose last result is `error` and every `return` of which gives zero values beside an error, or `nil` as the error |
| `comma_ok` | functions with results `(T, bool)` that always return `(zero, false)` or `(x, true)` |
| `read_only` | methods that write nothing rooted at their receiver, directly, through an alias, or through a call |
| `displays` | types with `Error() string` or `String() string`, and every enum or newtype over `string` |
| `unordered_iteration` | positions of `for … range` loops over a map |

A function holding a `foreign` statement gets none of `fallible`, `comma_ok` and `read_only`: what
the foreign source returns or writes is unknown. The exact rules are in the crate documentation of
[`crates/passes`](https://github.com/beyond10x/codespec/blob/main/crates/passes/src/lib.rs).

## Writers

### Go

`codespec-write-go` builds a `go/ast` tree from the module and prints it with `go/printer`, then
`go/format`. A `foreign` Go statement or variable initializer is spliced back verbatim. A `cir/1`
document that holds foreign Go is code: writing and building a document from an untrusted source
runs it. `foreign` source in any other language is refused. A construct Go cannot hold is refused
whole: exit 1, a message with its position, nothing written. Its output is gofmt-clean and
vetted, and the Go reader reads it back to the same `cir/1` (`write_test.go::TestOrdersFixedPoint`).

### Rust

`codespec-write-rust` writes a Cargo crate of `Cargo.toml` with no dependencies, `src/lib.rs` and
`refusals.json`, built as `syn` trees and printed with `prettyplease`. Every mapping is read off the
facts:

| cir/1 | Rust | Decided by |
|---|---|---|
| `*T` with `T` in `shared` | `Rc<RefCell<T>>` | `shared` |
| `*T` result, `T` not shared | `T`, moved out | `shared` |
| last result `error`, function in `fallible` | `Result<T, Error>` | `fallible` |
| `(T, bool)`, function in `comma_ok` | `Option<T>` | `comma_ok` |
| method in `read_only` | `&self`, else `&mut self` | `read_only` |
| type in `displays` | `impl fmt::Display` | `displays` |
| `chan<- any` | `std::sync::mpsc::Sender<Message>`, `enum Message` over the `messages` types | `messages` |
| a map loop at a position in `unordered_iteration` | a `BTreeMap`, iterated in key order | `unordered_iteration` |
| sentinel errors, error structs, formatted errors | one `enum Error` | — |
| Go `int` | `i64`, with wrapping arithmetic | — |

The writer refuses a module without facts whole, and the message gives the pass to run. On the
`stock` package of [Translate a Go package to Rust](/docs/guides/translate-go-to-rust), with
`<install>` the install directory:

```shell-session
$ codespec write --lang rust work/stock.read.cir.json --out work/no-facts
codespec-write-rust: module stock has no facts: the `facts` pass has not run; run `codespec pass` over it first
codespec: <install>/codespec-write-rust exited with exit status: 1
```

## Refusals

| stage | refused | result |
|---|---|---|
| reader | a construct `cir/1` cannot express | a `foreign` statement with source text, language, reason and position, and a `refusals` entry on the module |
| Rust writer, per function | a function it cannot write: a `foreign` statement, a construct without a mapping, a call of a refused function | left out, listed in `refusals.json` with its key, position and reason; exit 0 |
| writer, whole module | a module without facts, or one whose types cannot be written | exit 1, nothing written |

In `orders`, `Audit` starts a goroutine. The reader records
`{"pos":"orders.go:132","what":"go func() { …","reason":"goroutine: concurrency is outside cir/1"}`.
The Rust writer refuses exactly one function of `orders`: `Service.Audit`.

A refused function is absent from the output, so a harness trace cannot call it: the `orders`
trace format has no `audit` command for that reason.
