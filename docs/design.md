# codespec — design (2026-11-06)

## Why

An ESS specification describes a system from the outside: types, entities, lifecycles, commands,
events, views. What a function body computes is an obligation there
(`ess/docs/design/ess-structural-synthesis-obligations-realizations-design-v0.1.md`, "Arbitrary
business algorithm | Obligation"). Translating code between languages, or rewriting it within one,
needs a representation of the inside. `cir/1` is that representation.

A prior experiment (Go only, `cir/0`) lifted a 136-line Go order service, emitted Go and Rust from
it, and ran the original, the Go output and the Rust output on 200 seeded traces of 300 commands:
82,703 output lines, byte-equal across all three; a bug planted in the Rust output was caught at the
first seed. `cir/1` is `cir/0` with the changes in § 3.

## 1. Concepts

| concept | what |
|---|---|
| `cir/1` | the format: one canonical JSON document per module |
| reader | source language → `cir/1` |
| pass | `cir/1` → `cir/1`; deterministic, total, named |
| facts | whole-module analysis results written by passes and read by writers, so writers decide nothing |
| writer | `cir/1` → source language |
| refusal | a construct a reader or writer does not handle, with position and reason |
| harness | runs N implementations of one program on seeded traces and compares outputs byte for byte |

## 2. Languages

- Repository tooling (model, CLI, passes, harness, checks) is Rust.
- Readers and writers are **adapters**, each in the language whose tooling handles its target best.
  A Go writer builds `go/ast` and prints it with `go/printer`; a Rust writer can build `syn` trees and
  print them with `prettyplease`; the Go reader uses `go/types`.
- tree-sitter is the candidate where a language has no such tooling. It gives syntax trees only, so a
  tree-sitter reader also has to resolve names and infer types for its language. Whether that reaches
  the detail `cir/1` needs is measured, not assumed: a tree-sitter Go reader must produce
  byte-identical `cir/1` to the `go/types` reader on every example.
- An adapter is a program. A reader prints one module on stdout; a writer reads one on stdin.
  `codespec read` and `codespec write` validate each document against the model on the way through.

Two questions decide each adapter, and the harness answers both:

1. Does the reader capture enough detail to produce `cir/1`?
2. Does the writer produce code that behaves like the original?

## 3. `cir/1`

Defined by `crates/model/src/lib.rs`; `schemas/cir-1.schema.json` is generated from it.

| level | nodes |
|---|---|
| module | name, reader `{language, name, version}`, doc, types, consts, sentinel errors, funcs, layout, facts, refusals |
| types | `int {width: platform\|8\|16\|32\|64, signed}`, `bool`, `string`, `error`, `any`, `unit`, `named`, `ptr`, `slice`, `map`, `chan {dir}`, `tuple` |
| declarations | `struct`, `newtype`, `enum`, `union` |
| statements | `let`, `var`, `assign`, `incr`, `decr`, `if`, `for_range`, `switch`, `return`, `expr`, `send`, `lookup_else`, `foreign` |
| expressions | `lit`, `nil`, `var`, `const`, `errvar`, `field`, `index`, `call`, `method`, `binop`, `unop`, `struct`, `conv`, `make`, `zero`, `intrinsic`, `to_error` |

Changes from `cir/0`:

| change | why |
|---|---|
| Rust model is the definition, schema generated | one source of truth |
| `reader` provenance | a document says what produced it |
| `int {width, signed}` | `cir/0`'s `bits: 0` meant "Go `int`", a Go-only meaning |
| `pos` on every statement and declaration | refusals and diffs point at source |
| `union`, `switch` | Rust enums with data, Go `switch`, Rust `match` |
| every expression has a type, required | writers never infer |
| tagged nodes with strict fields | a document with an unknown field or kind is refused |

## 4. Passes (to port, M1)

| pass / fact | rule |
|---|---|
| `lookup-guard` | `v, ok := m[k]; if !ok { …return }` with `ok` unused elsewhere → `lookup_else` |
| `shared` | a reference to `T` stored in a container or field |
| `messages` | types sent on a channel of `any` |
| `fallible` | last result is an error and every error path returns zero values |
| `comma_ok` | `(T, bool)` always `(zero, false)` or `(x, true)` |
| `read_only` | a method writes nothing rooted at its receiver |
| `displays` | `Error()`/`String()` methods; string enums and newtypes |
| `unordered_iteration` | a loop over a map |

## 5. Milestones

| | content | done when |
|---|---|---|
| M0 | model, schema, CLI (`validate`, `schema`, `read`, `write`), Go reader, `orders` | `task check` green; `orders` read twice gives the same bytes |
| M1 | passes (Rust), Go writer (`go/ast`), Rust writer, harness | `orders`: original Go, Go output and Rust output byte-equal on 200 × 300 traces |
| M2 | tree-sitter Go reader | byte-identical `cir/1` to the `go/types` reader on every example, or the measured gap |
| M3 | a second, algorithmic example: loops, `switch`, slices, arithmetic | M1 and M2 criteria on both examples |
| M4 | Rust reader, Rust → `cir/1` → Go | harness green on a Rust example |

## 6. Later

- ESS: a specification supplies a service's skeleton, `cir/1` bodies fill its obligations, and the
  ESS conformance suite is an equivalence oracle beside the harness.
- More target languages through the same adapter boundary.
