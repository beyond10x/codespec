---
format: aep.planning-md/3
id: story:extern-ports
kind: story
status: draft
title: A writer writes an extern type and its calls as an interface with a stub
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-command
revision: 1
---
# Story: a writer writes an extern type and its calls as an interface with a stub

## Outcome

With `--ports`, the Go writer writes each extern type of a module as an interface holding the
extern methods the module calls on it, each extern function as a method of one `Ports`
interface, and a stub implementation that panics naming the extern. A slice that reaches a
database or another service through a library can then be written, built and run with an
implementation supplied behind the interface.

## Context

On release 0.8.0 all three writers refuse the slice of the Quarkus fixture at its first extern
type: Go `name "org.acme.CountryClient" is not a Go identifier`, Rust `this writer has no Rust
binding for a type outside the module`, C `type.extern: … is outside the module, and this
writer binds none`. `codespec effects` knows what those externs do (`db` for
`PanacheEntityBase.persist`, `net` for `CountryClient.byName`), and `codespec eval --stub`
answers for them, so the slice evaluates but cannot be written.

## Acceptance

- `codespec write --lang go --ports` over the checked-in slice of the Quarkus fixture exits 0
  with an empty `refusals.json`; the output builds and `go vet` passes.
- The output declares one interface per extern type (`CountryClient` with `ByName`) and a stub
  for each; the name mapping from a source path to a Go identifier is in the writer's doc.
- A Go test supplies implementations for the two externs, calls `create` and `origin`, and gets
  what `codespec eval` prints with the same answers as `--stub` values.
- Without `--ports` the writer's output and refusals are byte-equal to today's.
- A file beside the output lists each port with the effects `codespec effects` gives it.

## Out of Scope

- An implementation of any port (a database adapter, an HTTP client).
- The Rust and C writers.
- Transactions and injection: the flags stay data.

## Scope

- `adapters/go/cmd/codespec-write-go/`, `crates/cli/src/lib.rs` (passing the option),
  `conformance/` (a writer case), `CHANGELOG.md`. Inferred.
