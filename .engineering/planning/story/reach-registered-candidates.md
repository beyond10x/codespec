---
format: aep.planning-md/3
id: story:reach-registered-candidates
kind: story
status: draft
title: Reach narrows an indirect site to the functions registered where it calls through
relations:
- decomposes: epic:sound-reach
- depends_on: story:reach-at-positions
- depends_on: story:reader-indirect-sites-rust
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/reach.rs
- confidence: inferred
  path: docs/static-analysis.md
- confidence: inferred
  path: examples/reach-c/
- confidence: inferred
  path: examples/reach-rust/
revision: 7
---
# Story: reach narrows an indirect site to the functions stored where it calls through

## Outcome

`codespec reach` narrows two forms of `indirect` site (`story:reader-indirect-call-sites`) and
no other:

- a site whose `through` is a struct member or a parameter: its candidates are the functions
  the set stores there by a direct reference: assigned to that member by name, named in an
  initializer of that member (an array of structs included), or passed by name for that
  parameter;
- a `trait-object` site: the methods named `name` of the types made by `reached` or `unknown`
  code, the rule of `story:reach-three-valued`.

Each edge found this way is `inferred`: the report lists it under the function's
`inferred_edges` with the site's position and the position that stores the function or makes
the type, and `via` of a shortest path over it names `inferred`. `--at` records
(`story:reach-at-positions`) carry the same `inferred_edges`.

Every other site keeps every function that fits it by the first phase's rule: a `through` that
names a module variable or an interface, a site without a `through`, and the kinds `interface`,
`closure`, `fn-pointer` without a `through`, `reflection` and `dynamic-name`.

A member or parameter site also keeps every function that fits it when the stores cannot all be
seen as direct references:

- a store into the member, or an argument for the parameter, whose value is not a function's
  name: a variable, a parameter, another member, a call's result;
- a `foreign` statement in a `reached` or `unknown` function whose `src` text holds the
  member's name or its struct type's name;
- the struct, or a pointer to it or to the member, is passed to an extern no document declares
  or to a function without a body;
- the address of the member or of the struct is taken, or the struct is copied as a whole from
  a value the set does not construct;
- the type holding the member is declared by no document of the set.

## Acceptance

In `crates/effects/tests/reach.rs` unless named:

- `examples/reach-c` (extended here with a function of the table's signature that is stored
  nowhere; its `expected.json` changes): the function in the table is `reached` with an
  `inferred` edge naming the table initializer's line, and its `via` names `inferred`; the
  stored-nowhere function is `unreachable`.
- A callback case: a function passed by name for a parameter the receiving function calls is
  `reached` with an `inferred` edge naming the passing call's line.
- `examples/reach-rust` (its `expected.json` changes): the trait method of the type made in
  reached code is `reached` with an `inferred` edge naming the line that makes the type; the
  method of the type made only in a function nothing calls is `unreachable`; the method of the
  type made only in the function called through the stored closure stays `unknown`.
- One case per line of the two keep-everything lists above (five site forms, five store
  conditions): the stored-nowhere function, or the fitting method, is `unknown`. Among them: a
  `through` naming a module variable; a Go `interface` site through an interface name; a store
  `fp = f; s.cb = fp`; a function handed on through a wrapper's parameter.
- The property test of `story:reach-three-valued` holds with narrowing on, its reference
  interpreter extended with stores, and `task reach-examples` passes for all four examples.
- `crates/cli/tests/reach.rs`: an `--at` record in the table function carries `inferred_edges`.

## Scope

- Own: `crates/effects/src/lib.rs`, `crates/effects/src/set.rs`,
  `crates/effects/tests/reach.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/reach.rs`,
  `examples/reach-c`, `examples/reach-rust`, `docs/static-analysis.md`, `CHANGELOG.md`.
  Inferred; recorded as typed scope.
- Depends on `story:reach-at-positions` (the chain on the reach pass) and on
  `story:reader-indirect-sites-rust`, whose example it changes. No reader and no part of the
  model changes: every condition above is read from fields `cir/1` has after
  `story:reader-indirect-call-sites`.
