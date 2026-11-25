---
format: aep.planning-md/3
id: epic:release-diff-go-field-gaps
kind: epic
status: draft
title: Gaps a release review found in codespec's Go support
summary: read --spans on several init functions, per-program init roots for reach and diff, and unknown deltas grouped by reason
relations:
- informed_by: epic:release-fact-diff
- informed_by: epic:code-edit-language
- informed_by: epic:sound-reach
revision: 1
---
# Epic: Gaps a release review found in codespec's Go support

## Outcome

A release review of a Go service runs `codespec read --spans`, `codespec reach` and `codespec diff`
over two revisions, and each of three gaps it hit is closed:

- `codespec read --spans` succeeds on every package shape the Go reader reads. A span that names
  no node of the `cir/1` document is dropped with a reason and the rest of the spans document
  stays; the whole document is never lost.
- `codespec reach` and `codespec diff` take, as roots of an entry, the `init` functions and
  variable initializers of the packages the program holding that entry imports, not of every
  package of the set.
- `codespec diff` groups the deltas of kind `unknown-added` and `unknown-removed` by reason, so a
  reviewer reads counts instead of one line per delta, and each reason with 5 or more
  occurrences on a public Go corpus has a story of its own.

## Why

A release-review pipeline ran `codespec read --spans` and `codespec diff` over two revisions of a
Go service. It reported:

- `read --spans` failed with "names no node of the cir/1 document" on two package shapes, a command
  package of a `cobra` program and an internal metrics package; the pipeline then read without
  spans and no delta had a location;
- every Go package `init` is a root of every reach, so over a repository of several programs each
  entry reached about 975 functions, most of them reachable only from another program's packages;
- 234 of 293 deltas were of kind `unknown`, from callees not fully resolved (for example a method
  on a type of the same module).

None of the three is a defect of `codespec diff`'s design (`epic:release-fact-diff`) or of the
span format (`epic:code-edit-language`); each is a gap a real service showed. The stories are
drafted here, beside those epics, because they come from one consumer and one review.

## Stories

1. `story:go-spans-same-key-functions`: the likely cause of the first gap, several functions of one
   key in a package (`init`), inferred from the code and not run.
2. `story:read-spans-drops-unplaceable-spans`: whatever the cause, a span that names no node never
   costs the document.
3. `story:reach-inits-per-program`: per-program roots in `crates/effects`, used by `codespec reach`
   and `codespec diff`; the Go reader records the imports of a package for it.
4. `story:diff-unknown-reasons-measured`: the unknown reasons of `codespec diff` measured on three
   public Go projects at pinned commits, one draft story per reason with 5 or more occurrences.
5. `story:diff-unknowns-summary`: `codespec diff --unknowns summary`.

Stories 1 and 2 share `crates/edit` and are worked one after the other (2 after 1). Story 3 shares
`crates/effects/src/set.rs` with `story:reach-entry-file` and the stories of `epic:sound-reach`;
whichever starts second rebases on the other. Stories 4 and 5 start after `story:diff-command`,
which builds the command they measure and extend.

## Done When

- Each story is implemented.
- On a Go fixture of two programs sharing a library, `codespec reach` from the entry of one program
  reaches nothing that only the other program's packages reach (`story:reach-inits-per-program`).
- `codespec read --spans` over a package with two `init` functions exits 0 with the spans of both
  (`story:go-spans-same-key-functions`).
- `codespec diff --unknowns summary` over the pinned corpus prints one line per reason class with
  its count (`story:diff-unknowns-summary`).

## Out of scope

- A new entry kind or framework (`epic:sound-reach`, `story:go-http-entries`).
- Narrowing a call through an interface or function value (`story:reach-registered-candidates`).
- Any change to the weight of deltas of other kinds.
