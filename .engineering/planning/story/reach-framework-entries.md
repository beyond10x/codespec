---
format: aep.planning-md/3
id: story:reach-framework-entries
kind: story
status: draft
title: The Groovy reader records a Grails application's wiring as entries
relations:
- decomposes: epic:sound-reach
- depends_on: story:reach-registered-candidates
- depends_on: story:reader-indirect-sites-jvm
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/groovy/
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: examples/reach-grails/
revision: 6
---
# Story: the Groovy reader records a Grails application's wiring as entries

## Outcome

Reading a Grails application, the Groovy reader records in `Module.entries` (the entry list
`cir/1` already has, kind `http`) each controller action, each interceptor's `before` and
`after`, each filter closure and each target of a URL mapping, each as the function it reads
them as. A wiring target it cannot read as a function is a refusal at its line. There is one
source of framework entries, `Module.entries`; an entries file takes them with a `kind: http`
line (`story:reach-entry-file`). `codespec reach --list-entries` prints to standard output a
`codespec-entries/1` file: one name line per function the readers recorded in `Module.entries`,
with its kind in a `#` comment after it, and one comment line per refused wiring target with its
position. A person reviews it and passes it back with `--entries`.

## Acceptance

- `examples/reach-grails`, a small checked-in Grails application: one controller with the
  actions `list` and `show`, one interceptor with `before` only, one filter, one URL mapping
  whose target is `show`, and one service with one method that `show` calls.
  `--list-entries` prints the file in a checked-in expected file, which has four name lines
  (`list`, `show`, the interceptor's `before`, the filter) and no other name line.
- That printed file, passed as `--entries`, is read without an error; under it `show` has
  distance 0 and the service method has 1.
- A second example file with a filter the reader cannot read as a function: the printed file
  has a comment line with that filter's position and no name line for it.
- `Module.entries` needs no new kind: strict reading of the example's documents passes on the
  unchanged schema (`git diff --exit-code schemas/`).

## Scope

- Own: `adapters/groovy`, `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`,
  `crates/cli/tests/reach.rs`, `examples/reach-grails`, `CHANGELOG.md`. Inferred; recorded as
  typed scope.
- Depends on `story:reach-registered-candidates` (the chain on `crates/cli`) and on
  `story:reader-indirect-sites-jvm` (the same adapter). Other frameworks are their own stories.
