---
format: aep.planning-md/3
id: story:reader-groovy-call-lists-for-refused-code
kind: story
status: draft
title: The Groovy reader lists the calls of code it refuses
summary: Foreign statements carry call lists, refused methods become opaque functions, and two Grails 2 conventions resolve calls the type checker left open.
relations:
- depends_on: story:reader-groovy-aborts-on-grails-2
revision: 4
---
# Story: the Groovy reader lists the calls of code it refuses

## Outcome

A Groovy method or class the reader refuses still appears in the call graph. Every foreign
statement carries the calls its source makes, a refused method or a method of a refused class is
an `opaque` function with its call list, and two Grails 2 conventions give a target to a call the
static type checker left unresolved. `reach`, `cq` and `calls` then answer over a Grails
application whose bodies are mostly outside `cir/1`.

## Why

On 0.8.0 a refused method is a refusal row and nothing else: its calls are absent, so a function
only it calls looks unreached and a path through it ends. Reported by another session on
2026-11-11 from a read of one Grails 2.5.6 application (not named), 0.8.0 against an experimental
change of `adapters/groovy/src/codespec/read/groovy/Reader.java` (+284/−1, not in this
repository's history):

- functions 203 → 1032, of them 829 `opaque`;
- calls 13 → 5161;
- functions reached from the entries 921 → 1963;
- call sites with an unknown target 2352.

The numbers are as reported and were not reproduced here.

## What the experiment did

1. `calls` on every foreign statement: a visitor over the statement's AST lists, in source order,
   a `method` call for a target the type checker resolved (`DIRECT_METHOD_CALL_TARGET`), a `value`
   call for one it did not, a `new` for a constructor call and a `lambda` holding its own calls for
   a closure.
2. A method refused at its declaration, and every method and constructor of a class refused as a
   whole, is written as an `opaque` function with `reason` and `calls`.
3. Two rules for an unresolved call:
   - a GORM method called on a domain class of the module is a call of
     `grails.gorm.<Domain>.<method>`;
   - a call through a field named `xService`, where the module reads a class `XService`, is a call
     of that service's method (Grails 2 autowires such a field by name).

## Known flaws of the experiment, to settle in this story

- A rule edge is not marked: a reader of the document cannot tell a target the type checker
  resolved from one a naming convention gave. This needs a field on the call ref and so a `cir/1`
  minor (`schemas/cir-1.schema.json`, `website/docs/reference/cir-1.md`), or the rule edges stay out.
- Overloads of one name on one class share one `opaque` function, their calls concatenated.
- An unresolved call is a `value` call whose `sig` holds the method name as text; `cq` cannot
  select it by name; a Go function-value call has the same gap.
- The GORM method list is incomplete (`withNewTransaction` is missing).
- `adapters/groovy/README.md` does not yet name the two conventions among its assumptions.
- The experiment prints counters and stack traces to stderr; those lines do not belong in the
  reader.
- No fixture: every number above comes from an application that cannot be a test.

## Acceptance

- A made-up Grails 2 tree under `adapters/groovy/testdata/` with a refused method, a class refused
  as a whole, a closure, a GORM call on a domain class and a call through an `xService` field;
  its recorded document shows the `opaque` functions and their `calls`.
- `codespec reach` from that tree's entry reaches a service method called only from a refused
  method; on 0.8.0 the same command does not.
- Every call target that comes from a convention is distinguishable in the document from one the
  type checker resolved, and `codespec check` accepts the document.
- Two overloads of one name are either two functions or one function whose document says it
  stands for two; the choice is written in `adapters/groovy/README.md`.
- The documents of the existing Groovy fixtures change only by added `calls` and `opaque`
  entries; `conformance/` and `task check` exit 0.

## Depends on

`story:reader-groovy-aborts-on-grails-2`: both change `Reader.java`; this story starts from the
commit that closes that one.

## Evidence

Two commits on `impl/groovy-call-lists`: `0c11b657` (call lists and `opaque` functions) and
`5480cc12` (the two conventions, each marked by the `by` field of the `method` call ref, a `cir/1`
minor, and the fact `call_by`).

In this repository, on `5480cc12`: `ReaderTest` 97 passed, 0 failed, 0 skipped; model 66, effects
28 and query 21 passed; `task check` exit 0 (136 cargo suites, 1014 passed, 0 failed). The made-up
tree is `adapters/groovy/testdata/holds`.

Settled from the flaw list: convention edges are marked; two overloads are two functions
(`adapters/groovy/README.md`, "Overloads are functions of their own"); the GORM list holds every
method of the GORM API classes of the Grails 2.5.6 distribution; the README names both
conventions; the reader prints nothing to stderr. Still open: an unresolved call is a `value`
call whose `sig` holds the method name as text.

On the Grails 2.5.6 application the experiment was made on, run by the reporting session on
2026-11-11 with an adapter built from `0c11b657`; as reported, not reproduced here:

- the read exits 0 and writes its document;
- functions 1741, of them 203 read and 1538 `opaque` (203 and 0 on the commit before);
- calls 8285, of them 1269 linked to a function of the document; 5337 call sites with an unknown
  target;
- caller and callee pairs 1885.

`5480cc12` has not been run on that application yet.
