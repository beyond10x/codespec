---
format: aep.planning-md/3
id: epic:slice-to-service
kind: epic
status: draft
title: A service sliced out of a larger program by its entry points and written as a running service in another language
summary: Slice a module by entry functions, show the slice behaves like the original, write it in a target language and serve its endpoints; a system specification of the slice is an optional step beside that path.
relations:
- informed_by: epic:sound-reach
- informed_by: epic:whole-repo-to-syscalls
- informed_by: epic:codebase-transformations
- informed_by: epic:java-quarkus-reader
revision: 1
---
# Epic: A service sliced out of a larger program by its entry points and written as a running service in another language

## Outcome

Given the `cir/1` documents of a program and a list of entry functions (the endpoints of one
service), codespec produces a `cir/1` module that holds only what those entries reach, shows by a
run that it behaves like the original for those entries, writes it in the target language of
choice, and emits what turns the written code into a process that answers the same endpoints. The
use: take one service out of a monolith and run it in a preferred language.

A system specification (ESS) of the slice is an optional step beside this path, not a stage on
it: a projection of the slice's outside (types, entity, commands, views, events, errors) that a
contract, a conformance suite and a mutation audit can be derived from.

## Why Now

Filed from the operator's idea of 2026-11-11: an endpoint is an entry into a system; compute what
it reaches, take the union over a handful of endpoints as the surface, hold it as `cir/1`, lift
it to the specification of a service, harden that, and write it in several languages. A
feasibility experiment on release 0.8.0 with the shipped binaries gave the evidence below; the
stories are drafted only for the steps it supports.

## Evidence (release 0.8.0, `examples/orders` and the Quarkus fixture)

Subject one: `examples/orders` with the entries `New`, `Service.Place`, `Service.Pay`,
`Service.Get`. Subject two: `adapters/java/testdata/quarkus.read.cir.json` with its two `http`
entries.

- **Slice.** `codespec reach` gives the reached function names (4 of 9 for orders, 4 of 20 for
  the fixture) and nothing else; no command writes a module. A throwaway script outside the
  repository built one from the reach report: `codespec validate` accepted it (7 types, 4
  functions, 0 refusals) and `codespec pass` recomputed every fact list.
- **Reach misses a method with no call node.** `Service.Pay` returns `ErrWrongState{…}` as an
  error; reach lists `ErrWrongState.Error` as unreached. The slice without it validates and
  evaluates. The Go writer writes it and the result does not compile (`ErrWrongState does not
  implement error`); the Rust writer refuses `Service.Pay` (`ErrWrongState used as an error`);
  the C writer exits 1 (`ErrWrongState has no Error() string method`). With the method kept: 5
  functions.
- **Reach misses what a framework constructs.** In the fixture `NewFruitResource` and
  `NewFruitService` are unreached, so the slice cannot make the receiver of its own entries.
  `codespec reach` without `--entry` does not use the module's `entries`.
- **Same behaviour.** `codespec eval --session` over the nine calls of the checked-in session
  that name a chosen entry: original and slice print the same 22 lines, byte for byte. The
  evaluator never calls `Error()`, so this check alone did not show the missing method. The
  unsliced fixture does not evaluate at all (it stops in the initializer of a logger variable no
  chosen entry uses); its slice does, with two `--stub` answers.
- **Three languages.** The five-function slice writes as Go, Rust and C with no refusal; the
  unsliced module has one function (`Service.Audit`, a goroutine) that Rust and C refuse. Under
  `codespec harness`, with the example's drivers reduced to the chosen commands and the trace
  filtered to them: `200 seeds x 300 commands, 3 implementations (original, slice-go,
  slice-rust), 51965 output lines each, byte-equal`. The C output compiles; no orders driver
  exists for C. The harness cannot restrict a trace to a subset of commands.
- **Surface.** `codespec effects` reports no effect for the orders slice; its sends on the
  events channel appear only in the evaluator's trace. For the fixture's slice, with the Java
  reader's summary: `FruitResource.create` `db`, `FruitResource.origin` `net`.
- **Fault runs.** `codespec eval --session --each` on the slice gives the same traces as on the
  original, and shows that `Place` has changed two things and `Pay` one when the send fails.
- **Externs stop every writer.** All three writers refuse the fixture's slice at the extern
  type `org.acme.CountryClient.CountryClient`.
- **Running service.** 75 lines of hand-written Go around the unchanged Go output of the slice
  answered `POST /orders` (201, `{"id":"o-1"}`; 422 for a total of 0), `POST /orders/o-1/pay`
  (204, then 409 `order is paid`) and `GET /orders/o-1` (200). Each of those lines is a decision
  `cir/1` does not hold: method and path (the Go reader records no entry), where a parameter
  comes from, status per result and error, response encoding, one instance per process, a lock,
  the receiving end of the channel. `Entry` holds kind, function, method, path, `consumes` and
  `produces`, and no writer reads it.
- **ESS.** A specification of the slice written by hand (one entity with a two-state lifecycle,
  two commands, one view, two events, two errors) was valid on the first run of `ess specify
  validate`. Types, events, command inputs and errors mapped one to one from `cir/1` types and
  the facts `shared`, `messages`, `fallible`, `comma_ok`. The lifecycle (a guarded assignment to
  an enum-typed field), the guards as outcome conditions, the identity and the actor needed
  judgement. Not carried: the id format and its counter, error texts, the unknown-id branch,
  construction. ESS's interpreter refused a view over a field the slice never writes
  (`tracking`; only `Ship` wrote it), a true statement about the slice that no codespec command
  made; without the field its 8 synthesized scenarios passed and its mutation audit killed 8 of
  10 mutants with 2 stillborn. The server ESS synthesizes from that specification does not
  start: its unmet obligation is the assignment of the order id, which is code the slice holds.

## What this says about ESS in the middle

The path slice, check, write, serve runs on `cir/1` alone and keeps the bytes of the behaviour.
A path that went through a system specification as its only middle would drop them. The
specification earns its place beside the slice: as the contract of the service, and as the
source of a suite the written service can be held to. Its mechanical half is small and can be
generated; lifecycle and guard recognition is new analysis and is not drafted here beyond a
skeleton.

## Stories

In the order they are worked. 1 to 4 are the slice and its evidence; 5 to 7 end at the running
service; 8 and 9 are the optional specification step.

1. `story:reach-implicit-methods`: reach counts the `Error` method of a type used as an error,
   and the constructors a framework calls for entry receivers and injected beans.
2. `story:slice-command`: `codespec slice` writes the reached part of a module as a valid
   `cir/1` module.
3. `story:slice-equivalence`: one check that the slice and the original answer the same for the
   chosen entries, under the evaluator and under the harness.
4. `story:slice-three-languages`: the orders slice checked in, written as Go, Rust and C, each
   run under the harness.
5. `story:entry-bindings`: an entry carries where its parameters come from and how results and
   errors become responses; entries for a module whose reader records none.
6. `story:extern-ports`: a writer writes an extern type and its calls as an interface of the
   target language, with a stub.
7. `story:serve-writer-go`: the Go writer emits a server for a slice's `http` entries.
8. `story:slice-to-ess-skeleton`: the mechanical half of a system specification, projected from
   a slice.
9. `story:slice-held-to-ess-suite`: the served slice run against the suite ESS synthesizes
   from that specification.

## Out of Scope

- Three-valued reach, entry files and unresolved call sites: `epic:sound-reach`. This epic uses
  what it delivers and adds only the two edges of story 1.
- Slicing across a linked set of modules: after `epic:whole-repo-to-syscalls`; the first slice
  is one module.
- Rewrites of the slice other than removal: `epic:codebase-transformations`.
- Raising the share of functions a reader reads (`epic:reader-coverage`,
  `epic:groovy-grails-reader`, `epic:java-quarkus-reader`). A slice holding a `foreign`
  statement is reported, not repaired. The checked-in Grails fixture reads 14 of 38 functions.
- Object-relational mapping, transactions, dependency injection and configuration as generated
  code: the slice names them as externs and marks; story 6 gives them a port, not an
  implementation.
- Servers in Rust and C, and a Java writer.
- Recognising lifecycles and guards for the specification: story 8 lists them as gaps.

## Done When

- Stories 1 to 7 are implemented, and a checked-in slice of `examples/orders`, written as Go,
  answers the requests of story 7's acceptance.
- Stories 8 and 9 are implemented or archived with the reason.
