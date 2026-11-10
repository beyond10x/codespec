---
format: aep.planning-md/3
id: story:reader-indirect-call-sites
kind: story
status: draft
title: cir/1 states an unresolved call as a site with kind, signature and what it calls through; the C reader gives it
relations:
- decomposes: epic:sound-reach
- depends_on: story:readers-bound-untrusted-input
- depends_on: story:reach-entry-file
- depends_on: story:cir1-raise-out-of-entry
- depends_on: story:cir1-fields-of-imported-types
- depends_on: story:cir1-defer-receiver-root
- depends_on: story:reader-function-spans
- depends_on: story:reader-c-kit-conventions
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/c-read/
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/reach.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: examples/reach-c/
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 6
---
# Story: cir/1 states a call the reader could not resolve as a site with kind, signature and what it calls through; the C reader gives it

## Outcome

`cir/1` gains an optional field, decided here: `indirect` on a `foreign` statement and on an
expression a reader keeps foreign. `cir/1` stays `cir/1`; a document without the field reads and
writes as before. Each entry is one call the reader saw and could not resolve:

- `kind`: `fn-pointer`, `table`, `callback`, `trait-object`, `closure`, `interface`,
  `reflection`, `dynamic-name`;
- `name`: the method name the call names, where it names one;
- `params` and `results`: the call's types in `cir/1` type syntax, where the source gives them;
- `through`: what the call goes through, where the source names it: a struct member
  (`Type.member`), a parameter of the enclosing function, a module variable, or a trait or
  interface name.

What `indirect` says about its statement: a `foreign` statement that has a `calls` list is
unresolved only for its `indirect` entries; by giving both, the reader states that the statement
makes no other call. A `foreign` statement without a `calls` list fits every function, whatever
its `indirect` says. A call the reader lists in `calls` (a call by name, through an interface or
of a function value, which dispatch over the set resolves) gets no `indirect` entry.

`codespec reach` decides which functions a site fits by a rule per source language, each at
least as wide as what that language lets a program call there. `n` is the site's number of
parameters. In every language `reflection` and `dynamic-name` fit every function, and so does a
site without `params`.

- C: every function with `n` parameters, types ignored (a cast pointer is callable), and every
  variadic function.
- Rust, `fn-pointer` and `closure`: every function with `n` parameters, every method with `n`
  or `n - 1` parameters (a method used as a value takes its receiver first), and every closure
  the set uses as a value. `trait-object`: every method named `name` on any type of the set,
  whatever its parameter count.
- Go, `fn-pointer`: every function with `n` parameters and every method with `n` or `n - 1`
  (a method value binds its receiver, a method expression takes it first); a variadic function
  or method fits when it has at most `n + 1`. `interface`: every method named `name`.
- Java and Groovy, `interface`: every method named `name`, whatever its parameter count
  (varargs, default arguments), every lambda and closure of the set, and every method the set
  uses as a method reference (its name need not be the interface's). `closure`: every lambda
  and closure of the set and every method used as a method reference.

The C reader gives `indirect` for a call through a function pointer variable or parameter
(`fn-pointer`), through a struct member (`table`, with `through`), through an element of an array
of such structs (`table`), and for passing a function to a function that stores or calls it
(`callback`: the site is the call inside the receiving function, `through` its parameter).

The Rust, Go, Java and Groovy readers follow in `story:reader-indirect-sites-rust`, `-go` and
`-jvm`; the matching rules for their languages are built and tested here, on written documents.

## Acceptance

- `docs/design.md` records the decision (an optional field of `cir/1`), the field's parts, what
  `indirect` beside `calls` states, and the matching rule per language.
- `crates/model/tests/model.rs`: strict reading accepts a document with `indirect` and one
  without; it refuses an unknown `kind`, an empty `through`, an empty `name`, and a `params`
  entry that does not parse as a `cir/1` type. After `task schema`, `schemas/cir-1.schema.json`
  has the field and `git diff --exit-code schemas/` is clean on the committed tree.
- `adapters/c-read/tests`: four checked-in C files, one per form the C reader gives as listed
  above (`fn-pointer` by variable, `table` by member, `table` by array element, `callback`),
  each with its expected `indirect` entries.
- `crates/effects/tests/reach.rs`, one test per language's rule on written documents marked
  with that language:
  - C: with a site of two parameters, an uncalled function of one parameter is `unreachable`,
    one of two parameters of other types is `unknown`, a variadic one is `unknown`; with a site
    without `params` all are `unknown`.
  - Rust: with an `fn-pointer` site of two parameters, a method of one parameter is `unknown`
    (receiver first) and a function of three is `unreachable`; with a `trait-object` site named
    `run`, a method `run` of any parameter count is `unknown` and a method `stop` is
    `unreachable`.
  - Go: with an `fn-pointer` site of one parameter, a method of no parameters and one of one
    are `unknown`, a function of three is `unreachable`; a `reflection` site makes all
    `unknown`.
  - Java: with an `interface` site named `compare`, a method `compare` of three parameters, a
    lambda and a method used as a method reference under another name are `unknown`; a method
    `other` that nothing uses as a reference is `unreachable`.
- Same file: a `foreign` statement with `calls` and one `indirect` entry of two parameters
  leaves a function of one parameter `unreachable`; the same statement without `calls` makes it
  `unknown`.
- Ground truth: `examples/reach-c` is a C program checked in with its entry list and the
  expected report `expected.json`. A gate step (`task reach-examples`, added here, a step of
  `check`) compiles it with a line printed at each function's start, runs it, requires every
  function the run printed to be `reached` or `unknown` in `codespec reach` over the read, and
  compares the report with `expected.json`. The program calls through a cast function pointer,
  a `void *` callback and a table of structs. In `expected.json` a function nothing could call
  is `unreachable`, a function called only through the table is `unknown` with the call site's
  position, and a named function has distance 3 from the entry.
- The reader conformance reports do not change: `git diff --exit-code
  website/data/conformance/reader-*.json` after `task docs-generate`, and `task adapters-check`
  passes.

## Scope

- Own: `crates/model/src/lib.rs`, `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `adapters/c-read`, `crates/effects/src/lib.rs`, `crates/effects/src/set.rs`,
  `crates/effects/tests/reach.rs`, `examples/reach-c`, `Taskfile.yml`, `docs/design.md`,
  `AGENTS.md`, `CHANGELOG.md`. Inferred; recorded as typed scope.
- `crates/model/src/lib.rs` and the schema are also edited by `story:cir1-raise-out-of-entry`,
  `story:cir1-fields-of-imported-types` and `story:cir1-defer-receiver-root`; `adapters/c-read`,
  `docs/design.md` by `story:reader-function-spans`, `story:readers-bound-untrusted-input` and
  `story:reader-c-kit-conventions`. This story depends on each of them for that reason only.
- Runs after `story:reach-entry-file` (the reach pass in `crates/effects`).
