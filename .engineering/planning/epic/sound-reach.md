---
format: aep.planning-md/3
id: epic:sound-reach
kind: epic
status: draft
title: codespec reach answers reached, unreachable or unknown, never unreachable for a function an unread call could reach
relations:
- depends_on: epic:whole-repo-to-syscalls
revision: 5
---
# Epic: codespec reach answers reached, unreachable or unknown, and never unreachable for a function an unread call could reach

## Outcome

Given a set of `cir/1` documents and a reviewed list of entry functions, `codespec reach` says for
every function one of three things, computed without a language model:

- `reached`: a path of resolved calls from an entry exists; with its length in calls (`distance`)
  and the entry it starts from (`nearest_entry`).
- `unreachable`: no path exists, and no call site the readers could not resolve could call it.
- `unknown`: not reached by resolved calls, but a call site with no resolved target that this
  function fits is in code that is reached or itself `unknown`.

For a `file:line` handed to it, it gives the status of the function holding that line, or one of
two answers that are not a status: `no function` (no function of the set holds the line) and
`not read` (the reader refused the construct there).

The invariant: `unreachable` is never given for a function that an unresolved call site it fits
could call. `unknown` here is what the requirement calls `unknown-indirect`; the name is shorter
because a function without a body and a `foreign` statement are unresolved sites too.

## Why Now

Filed from the operator's requirement of 2026-11-10 (relayed by a peer session; requirements R1 to
R7). Today `codespec reach` (`story:reachability`) is two-valued: `codespec-reach-report/1` lists
functions reached and not reached. A reader keeps a call through a C function pointer, a Rust
`dyn Trait` or stored closure, or a reflective JVM call inside a `foreign` statement, and a
`foreign` statement without a call list gives the call graph no edge. A function called only from
there is listed as not reached, which a user reads as dead or as safe from input. `codespec
effects` already lists such a site as `unknown` with a position; `codespec reach` does not.

## What exists

- The linked call graph with dispatch over the set, `cha` and `rta` (`story:link-modules`,
  `story:dispatch-over-set`), in `crates/effects`.
- `codespec reach --entry`, `--exported`, `--dispatch`, `--report` (`story:reachability`).
- `foreign` statements with an optional list of the calls they make.
- `cir/1` `Module.entries`: functions a framework starts, of kind `http`, `scheduled` or
  `incoming`, recorded by a reader.

## Stories, in the order they are worked

The reach pass lives in `crates/effects` and `crates/cli`, and the stories that edit it form a
chain, each depending on the one before: 1, 2, 3, 7, 8, 9. The three reader stories (4 to 6) own
different adapters, depend on the third only, and can be worked beside each other and beside 7.

1. `story:reach-three-valued`: the three statuses, distance and nearest entry, from what the
   readers give today (a `foreign` statement without a call list fits every function).
2. `story:reach-entry-file`: entries as a reviewed data file, by name, by pattern or by the kind
   a reader recorded, with starter files per language.
3. `story:reader-indirect-call-sites`: the `indirect` field of `cir/1`, the matching rule per
   language, and the C reader giving it.
4. `story:reader-indirect-sites-rust`, 5. `story:reader-indirect-sites-go`,
   6. `story:reader-indirect-sites-jvm`: the same for the Rust, Go, and Java and Groovy readers,
   each with its language's checked-in example.
7. `story:reach-at-positions`: the answer for a list of `file:line` positions, for a set of
   findings.
8. `story:reach-registered-candidates` (second phase): a site's candidates narrowed to the
   functions stored where it calls through, each such edge `inferred` with the position that
   stores it.
9. `story:reach-framework-entries` (third phase): the Groovy reader records a Grails
   application's controller actions, interceptors, filters and URL-mapping targets as entries.

## Out of Scope

- Data flow from an entry's input to a sink: this is call reachability only.
- A language model in any step.
- Guessing entries: with no entry given and no `main.main`, `reach` refuses as it does today.
- Narrowing in the second phase beyond a C struct member or parameter and a Rust trait object.
  A Go or Java `interface` site, a `closure` or `fn-pointer` site without a `through`, and every
  `reflection` and `dynamic-name` site keep the width of the first phase; narrowing any of them
  is a later story nobody has drafted.

## Done When

- Each story is implemented. Each reader story (3 to 6) checks in its language's example with a
  function called only through an unresolved site that is `unknown`, a function nothing could
  call that is `unreachable`, and a stated distance from an entry to a named function;
  `story:reach-registered-candidates` turns the C and Rust `unknown` of those examples into
  `reached` with an `inferred` edge.

## Open questions (not decided here)

- Whether a reader drops a call without a `foreign` statement or a refusal. The invariant holds
  only as far as the readers state what they did not read; `story:reader-check-evaluator-step`
  checks reads by running them, and no story of this epic audits it further.
- A Grails application's dynamic finders are `dynamic-name` sites that fit every function, so
  most of its unreached functions are `unknown`. No story of this epic narrows them.
