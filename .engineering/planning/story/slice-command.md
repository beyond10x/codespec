---
format: aep.planning-md/3
id: story:slice-command
kind: story
status: draft
title: codespec slice writes the part of a module its entries reach as a valid cir/1 module
relations:
- decomposes: epic:slice-to-service
- depends_on: story:reach-implicit-methods
revision: 1
---
# Story: codespec slice writes the part of a module its entries reach as a valid cir/1 module

## Outcome

`codespec slice --entry <FUNC>… --out <OUT> <MODULE>` writes a `cir/1` module, in canonical
encoding, holding the functions `codespec reach` lists as reached for those entries and exactly
what they need, so that the slice can be validated, passed, evaluated and written like any
module.

## Context

`codespec reach` prints names only. A throwaway script built the module for `examples/orders`
and the Quarkus fixture; these are the decisions it had to guess and this command has to make:

| part of the module | rule |
|---|---|
| `funcs` | the reached functions (`story:reach-implicit-methods` included) |
| `types` | every type a kept function, type, constant or variable names, to a fixed point; a type is kept whole |
| `consts`, `errors`, `vars` | those a kept function names; a variable's initializer is kept with it |
| a variable no kept function names | dropped with its initializer, and listed in the report as dropped, since `reach` counts initializers as entries |
| `externs` | those a kept function or initializer calls |
| `layout` | filtered, order kept |
| `refusals` | those at the position of a `foreign` statement in a kept function |
| `entries`, `beans`, `injects`, `transactional`, `persistence` | those naming a kept function or type |
| `inits`, `interfaces`, `methods`, `opaque`, `linknames` | filtered by the same keys |
| `facts` | removed; `codespec pass` writes them |

## Acceptance

- `crates/cli/tests/slice.rs`: `codespec slice --entry orders.New --entry orders.Service.Place
  --entry orders.Service.Pay --entry orders.Service.Get` over
  `examples/orders/orders.read.cir.json` gives a module with 5 functions (`ErrWrongState.Error`
  among them), 7 types, 0 refusals, equal to a checked-in file; `codespec validate --canonical`
  exits 0 on it; `codespec pass` on it gives `messages` of `OrderPlaced` and `OrderPaid` only.
- Over the Quarkus read with its two `http` entries and `--framework-roots`: the slice has no
  `vars`, two `externs`, two `entries`, one `beans` item and the `persistence` mark of `Fruit`
  only, equal to a checked-in file.
- Standard error lists every dropped variable with its position.
- The same input gives the same bytes; an entry naming no function is refused as `reach` does.
- A module with facts is accepted and written without them.

## Out of Scope

- Several modules in one slice.
- Narrowing a type to the variants or fields the slice uses.

## Scope

- `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/tests/slice.rs`, a new module
  in `crates/effects/src/` or `crates/passes/src/`, `website/docs/reference/cli.md` (generated),
  `CHANGELOG.md`. Inferred.
