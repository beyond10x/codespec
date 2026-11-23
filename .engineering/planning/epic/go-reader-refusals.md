---
format: aep.planning-md/3
id: epic:go-reader-refusals
kind: epic
status: active
title: Go reader refuses less of a real service module
summary: A real Go service module reads with most functions read; progress is functions read, not refusals removed.
relations:
- depends_on: story:cir1-fields-of-imported-types
- depends_on: story:cir1-defer-receiver-root
- depends_on: story:cir1-type-test
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:48Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:48Z", actor: "human:timo", revision: 3}
---
# Epic: Go reader refuses less of a real service module

## Outcome

`codespec-read-go` reads a real Go service module (35 packages, ~1,336 functions) with most
functions read and few refusals left, each refusal naming the sub-shape it refused. Progress is
measured as functions read and statements kept (`codespec measure`), not as refusals removed:
a function refused for its signature hides its body, so fixing a signature unmasks refusals
inside the body and the refusal count can rise while more is read.

## Why Now

A consumer session read a private Go service module with codespec 0.12.0 and got about 1,740
refusals from the Go reader. Its counts per reason:

| reason | records |
|---|---|
| expression outside cir/1 | 806 |
| receiver type with no cir/1 type | 309 |
| if with an init statement | 129 |
| defer is outside cir/1 | 92 |
| declaration other than `var x T` | 84 |
| named results kept in a closure | 53 |
| initializer outside cir/1 | 52 |
| blank variable | 46 |
| field of a type with no cir/1 type | 42 |
| type outside cir/1 | 30 |
| parameter type with no cir/1 type | 16 |
| select | 9 |
| type switch | 7 |
| goroutine | 7 |
| type alias | 6 |
| switch with an init statement | 2 |
| generic-type shapes | ~50 |

The module is private: nothing of it is read or committed here. Every reproducer is written from
scratch with generic names. A public proxy corpus of 17 standard-library packages that look like
service code (`net/http`, `database/sql`, `log/slog`, `encoding/json`, `text/template`,
`crypto/tls` and 11 more) has a close reason mix (3,061 refusals) and is the measurement here.

## Waves

1. Reader only, no change of `cir/1` (one PR): the refusal names the sub-shape of a refused
   expression; `struct{}` read as `unit`; blank assignments and compile-time-only declarations;
   every local `var` and `const`; `if` and `switch` inits hoisted with fresh names; named results
   without `defer`; small literal and builtin forms (positional and slice literals, `make` of a
   map, `new`, empty literals of other packages' structs, a `nil` argument, constants of other
   packages).
2. Additive `cir/1` features, one model wave, serialized: fields of other modules' structs
   (`story:cir1-fields-of-imported-types`), `defer` (`story:cir1-defer-receiver-root`), conversion
   to basic and extern types, constraints and variadic func types, slice expressions, variables of
   other packages, `make` with a length and the remaining builtins.
3. The rest: type tests (`story:cir1-type-test`), anonymous struct and interface types, local type
   declarations, map and array literals, generic interfaces, `uintptr`, `select`, named results
   with `defer`, effects of function values made by an extern.

## Done When

- Every story of waves 1 to 3 that decomposes this epic is implemented.
- On the proxy corpus, functions read rise with each wave; each wave's PR states the counts of
  refusals per reason and of functions read before and after.

## Out of Scope

- A change of `cir/1` meaning or encoding (`cir/2`).
- Goroutines (`go f()`): 7 records, and their calls are already listed for reach and effects.
- `recover()`, complex numbers.
