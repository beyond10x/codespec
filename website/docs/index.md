---
slug: /
title: Overview
sidebar_position: 1
description: What codespec is, what it is not, and where it sits among its neighbours.
---

# Overview

codespec is a typed code intermediate representation, `cir/1`, and the tools around it. A
**reader** lifts a source language into `cir/1`, **passes** rewrite it and compute facts about the
whole module, and a **writer** emits a source language from it. A **harness** runs the original
and every output on the same seeded inputs and compares what they print, byte for byte.

```text
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
                                                         │
original ───────────────── harness: byte-equal? ─────────┘
```

This tree reads Go, Rust, Java and Groovy, writes Go and Rust, and runs the harness on two examples, `orders`
in Go and `inventory` in Rust. For each, the original, the Go output and the Rust output agree on
200 traces of 300 commands each. [Status](/docs/status) lists every capability and the test that
checks it. [Vision](/docs/vision) says where it is going.

## What it is

| part | what it is | page |
|---|---|---|
| format | `cir/1`: one canonical JSON document per module with types, constants, sentinel errors, functions, statements and expressions, every expression with its type. The Rust model in `b10x-codespec-model` is the definition, and the [JSON Schema](/docs/reference/cir-1) is generated from it. | [The cir/1 model](/docs/concepts/cir-1) |
| command | `codespec` validates documents, applies the passes, runs reader and writer adapters, checks everything that crosses to and from them, and runs the harness. | [CLI reference](/docs/reference/cli) |
| adapters | each reader and writer is a separate program in the language whose tooling handles its target best: `codespec-read-go` and `codespec-write-go` in Go on `go/types` and `go/ast`, `codespec-read-rust` and `codespec-write-rust` in Rust on `syn`, `codespec-read-java` on javac's attributed trees, `codespec-read-groovy` on the Groovy 2.4 compiler and its static type checker, `codespec-read-c` in Rust on the typed AST clang prints | [Readers, passes and writers](/docs/concepts/pipeline) |
| evidence | a translation is accepted when the harness finds no differing byte | [The harness](/docs/concepts/harness) |
| analysis | `codespec effects` reports per function the effects it may have, such as environment, file system, network and clock, from summaries of the library functions it calls. It lists every call it cannot account for as unknown. Over a whole repository read with its imports, it reports the syscalls each function may reach. | [Static analysis](/docs/concepts/static-analysis) |

## What it is not

- Not a language model and not a guesser: nothing in the pipeline is a model. A construct a
  reader cannot express is kept verbatim as a `foreign` statement with a reason and listed as a
  refusal. A writer that cannot write a function leaves it out and lists it.
- Not a general-purpose transpiler today: this tree reads Go, Java, Groovy and the subset of Rust
  that `cir/1` covers with the [Rust reader](/docs/concepts/pipeline) on `syn`. Concurrency (goroutines), network I/O and
  anything else `cir/1` has no node for are refused, not approximated.
- Not a test generator for any program: the harness has two trace formats, `orders` and
  `inventory`. A new program needs its own drivers and trace format to be compared.
- Not a specification of a system's outside: entities, commands, events and lifecycles belong
  to ESS. `cir/1` describes what code does inside a function.

## Neighbours

| Component | Relation |
|---|---|
| [ESS](https://beyond10x.github.io/ess/) ([GitHub](https://github.com/beyond10x/ess)) | Specifies a system from the outside. Not used by codespec today; later, an ESS specification may supply a service's skeleton whose bodies `cir/1` fills, with the ESS conformance suite as an equivalence oracle beside the harness. |

## Start

- [Use cases](/docs/use-cases): why you would use codespec
- [Getting started](/docs/getting-started)
- [Showcase](/docs/examples/greeter)
- [Showcase: series](/docs/examples/series)
- [Translate a Go package to Rust](/docs/guides/translate-go-to-rust)
- [Map a repository to syscalls](/docs/guides/map-a-repository-to-syscalls)
- [netcheck: down to syscalls](/docs/examples/netcheck)
