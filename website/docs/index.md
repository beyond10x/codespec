---
slug: /
title: codespec
sidebar_position: 1
description: What codespec is, what it is not, and where it sits among its neighbours.
---

# codespec

codespec is a typed code intermediate representation, `cir/1`, and the tools around it. A
**reader** lifts a source language into `cir/1`, **passes** rewrite it and compute facts about the
whole module, and a **writer** emits a source language from it. A **harness** runs the original
and every output on the same seeded inputs and compares what they print, byte for byte.

```text
source ──reader──▶ cir/1 ──passes──▶ cir/1 ──writer──▶ source
                                                         │
original ───────────────── harness: byte-equal? ─────────┘
```

This tree reads Go and Rust, writes Go and Rust, and runs the harness on two examples: for
`orders` the original Go, the Go output and the Rust output, for `inventory` the original Rust,
the Go output and the Rust output, agree on 200 traces of 300 commands each.
[Status](/docs/status) lists every capability and the test that holds it; [Vision](/docs/vision)
says where it is going.

## What it is

- **A format.** `cir/1` is one canonical JSON document per module: types, constants, sentinel
  errors, functions, statements and expressions, every expression with its type. The Rust model in
  `b10x-codespec-model` is the definition; the [JSON Schema](/docs/reference/cir-1) is generated
  from it. See [The cir/1 model](/docs/concepts/cir-1).
- **A command.** `codespec` validates documents, applies the passes, runs reader and writer
  adapters and checks everything that crosses to and from them, and runs the harness
  ([CLI reference](/docs/reference/cli)).
- **Adapters.** Each reader and writer is a separate program, written in the language whose tooling
  handles its target best: `codespec-read-go` and `codespec-write-go` in Go on `go/types` and
  `go/ast`, `codespec-write-rust` in Rust on `syn` and `prettyplease`
  ([Readers, passes and writers](/docs/concepts/pipeline)).
- **Evidence.** A translation is accepted when the harness finds no differing byte, not when it
  looks right ([The harness](/docs/concepts/harness)).
- **An analysis.** `codespec effects` reports per function the effects it may have (environment,
  file system, network, clock, …) from summaries of the library functions it calls, and lists
  every call it cannot account for as unknown ([CLI reference](/docs/reference/cli)).

## What it is not

- **Not a language model and not a guesser.** Nothing in the pipeline is a model. A construct a
  reader cannot express is kept verbatim as a `foreign` statement with a reason and listed as a
  refusal; a writer that cannot write a function leaves it out and says so.
- **Not a general-purpose transpiler today.** This tree reads Go, and the subset of Rust that `cir/1`
  covers with the [Rust reader](/docs/concepts/pipeline) on `syn`. Concurrency (goroutines), network I/O and
  anything else `cir/1` has no node for are refused, not approximated.
- **Not a test generator for any program.** The harness has two trace formats, `orders` and
  `inventory`. A new program needs its own drivers and trace format to be compared.
- **Not a specification of a system's outside.** Entities, commands, events and lifecycles belong
  to ESS; `cir/1` describes what code does inside a function.

## Neighbours

| Component | Relation |
|---|---|
| [ESS](https://beyond10x.github.io/ess/) ([GitHub](https://github.com/beyond10x/ess)) | Specifies a system from the outside. Not used by codespec today; later, an ESS specification may supply a service's skeleton whose bodies `cir/1` fills, with the ESS conformance suite as an equivalence oracle beside the harness. |

## Start

- [Getting started](/docs/getting-started): build, read `orders`, pass, write Go and Rust, run the
  harness.
- [Showcase](/docs/examples/greeter): one small Go program, its `cir/1`, and the Go and Rust written
  from it.
- [Showcase: series](/docs/examples/series): an algorithm with loops, `switch` and overflowing
  arithmetic, its `cir/1` drawn as a tree, and the Rust written from each construct.
- [Translate a Go package to Rust](/docs/guides/translate-go-to-rust).
