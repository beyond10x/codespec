---
format: aep.planning-md/3
id: story:reach-three-valued
kind: story
status: draft
title: codespec reach reports reached, unreachable or unknown, with distance and nearest entry
relations:
- decomposes: epic:sound-reach
- depends_on: story:reachability
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
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
  path: website/docs/reference/cli.md
revision: 5
---
# Story: codespec reach reports reached, unreachable or unknown, with distance and nearest entry

## Outcome

`codespec reach` prints `codespec-reach-report/2` and no longer `codespec-reach-report/1`: per
function `status` (`reached`, `unreachable`, `unknown`), for `reached` the `distance` (fewest
calls from an entry; an entry is 0), `nearest_entry`, and `via`, the kinds of edge on that
shortest path (`call`, `dispatch` with the candidate count, `value` for a function reached by
being used as a value). For `unknown`, `sites`: the positions of the unresolved sites that could
call it.

An unresolved site, and what it fits until `story:reader-indirect-call-sites` narrows it:

- a `foreign` statement without a call list: every function of the set;
- a call of a function without a body: every function of the set (its body, which nobody read,
  can name any of them);
- a call of an extern no document of the set declares: what the set hands to it. That is each
  function value passed; every method of the type of each value passed, and when that type is an
  interface every method of every type of the set that implements it; and every function a
  function-typed field of a passed struct can hold (by the dispatch rule for a value of that
  type), through fields of fields. A call that passes none of these is not a site.

A site counts only when its function is `reached` or `unknown`.

Unknown spreads through everything the reach pass uses: every function reached by resolved calls
from an `unknown` function is `unknown` unless it is `reached`; and under `--dispatch rta` the
types an `unknown` function makes and the function values it uses count as made and used, so a
method that is a dispatch candidate only through a type made in an `unknown` function is
`unknown`, not `unreachable`.

## Acceptance

All in `crates/effects/tests/reach.rs` unless named otherwise.

- A function called only from a `foreign` statement without a call list, in a function reached
  from the entry, is `unknown` and its `sites` names that statement's position.
- The same with the site being a call of a function without a body.
- An undeclared extern, one case each: passed a function value, that function is `unknown`;
  passed a value of a type of the set, that type's methods are `unknown` and a method of a type
  not passed stays `unreachable`; passed an interface value, the methods of both implementing
  types are `unknown`; passed a struct with a function-typed field, the function stored in the
  set for that field's type is `unknown`; passed only an integer, nothing becomes `unknown`.
- In a set with no unresolved site, a function nothing calls is `unreachable`.
- An unresolved site in a function that is itself `unreachable` makes nothing `unknown`.
- Spread by calls: `u` is `unknown`, `u` calls `v`, nothing else calls `v`: `v` is `unknown`.
- Spread under `rta`: type `T` is made only in an `unknown` function and implements an interface
  a reached function calls through: `T`'s method is `unknown`; with the making function
  `unreachable` it is `unreachable`.
- Distances on a chain of three calls are 0, 1, 2, 3; with two entries the nearer one is
  `nearest_entry`, ties broken by entry name; a path through an interface call lists `dispatch`
  with its candidate count; a function reached only as a value lists `value`.
- A property test over generated sets (calls, interface calls, types made, values used,
  unresolved sites of each of the three forms): a reference interpreter in the test computes the
  functions callable from the entries, where a site of the first two forms calls every function
  and makes every type, and an extern site calls what is handed to it; each of them is `reached`
  or `unknown` in the report, under `cha` and under `rta`.
- `crates/cli/tests/reach.rs`: the output's first line names `codespec-reach-report/2`, and the
  output is byte-identical across two runs.
- `docs/static-analysis.md` states the three statuses, the unresolved sites, both spread rules
  and the invariant; the CLI reference is regenerated (`task docs-check` passes).

## Scope

- Own: `crates/effects/src/lib.rs`, `crates/effects/src/set.rs`, `crates/effects/tests/reach.rs`,
  `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/tests/reach.rs`,
  `docs/static-analysis.md`, `website/docs/reference/cli.md` (generated), `CHANGELOG.md`.
  Inferred; recorded as the story's typed scope.
- `crates/cli/src/lib.rs` and `main.rs` are also edited by `story:mcp-server-read-tools` and
  `story:reader-function-spans`; `story:mcp-server-read-tools` depends on this story for that
  reason only.
