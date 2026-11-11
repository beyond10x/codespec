---
format: aep.planning-md/3
id: story:serve-writer-go
kind: story
status: draft
title: The Go writer emits a server for a slice's http entries
relations:
- decomposes: epic:slice-to-service
- depends_on: story:entry-bindings
- depends_on: story:extern-ports
- depends_on: story:slice-three-languages
revision: 1
---
# Story: the Go writer emits a server for a slice's http entries

## Outcome

`codespec write --lang go --serve` writes, beside the package, a `main` that serves every
`http` entry of the module: it routes by the entry's method and path, reads each parameter from
its recorded source, calls the function, and answers with the recorded status and encoding. A
slice with bound entries becomes a process that answers its endpoints, with no hand-written
code.

## Context

On release 0.8.0, 75 lines of hand-written Go around the unchanged Go output of the orders
slice gave a process that answered `POST /orders` with 201 and `{"id":"o-1"}`, a total of 0
with 422 and `total must be positive`, a second `POST /orders/o-1/pay` with 409 and `order is
paid`, and `GET /orders/o-1` with the order. Besides the bindings (`story:entry-bindings`),
that code decided: one instance of the receiver per process and how it is made, a lock around
every call (the module has none), and what receives the values the module sends on its channel.

## Acceptance

- Over `examples/orders-slice` with its entries file: the written server builds with the Go
  standard library only; a test starts it on a local port, sends the six requests above and
  compares status and body with a checked-in file.
- The receiver of the entries is made once, by the function the entries file names as its
  constructor; every call holds one lock; values sent on a channel parameter of the constructor
  are written one JSON object per line to standard error. Each of the three is stated in the
  writer's doc.
- A module with an `http` entry lacking a parameter source or a status is refused, naming the
  entry.
- With `--ports` (`story:extern-ports`), the server takes the port implementations from one
  function the user fills in, and with the stubs it starts and answers 500 naming the extern
  for an entry that reaches one. A test over the Quarkus slice.

## Out of Scope

- `scheduled` and `incoming` entries.
- Servers in Rust and C.
- Persistence of the module's state across restarts.

## Scope

- `adapters/go/cmd/codespec-write-go/`, `examples/orders-slice/`, `Taskfile.yml`,
  `website/docs/guides/`, `CHANGELOG.md`. Inferred.
