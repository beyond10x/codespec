---
format: aep.planning-md/3
id: story:java-reader-nullable-extern-string
kind: story
status: draft
title: The Java reader reads a null test of a string a jar method returns
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: the Java reader reads a null test of a string a jar method returns

## Outcome

The Java reader reads `x.getMessage() != null` and the use of that string in the guarded branch,
where the method is a jar's and returns `String`, without mapping a possibly null reference to a
`cir/1` string that has no null.

## Why

Measured by `story:cir1-raise-out-of-entry` on the Quarkus quickstart: 2 of its 8 remaining
refusals are `exception.getMessage() != null`, one in each exception mapper. A jar's `String` maps
to a `cir/1` string, which has no null, so the reader refuses the comparison.

## Acceptance

- The design states the form: the result of an extern call returning a reference `String` is an
  optional string where the source tests it against null, and a plain string where it does not;
  or another form the design names, with its reason.
- The Java reader reads `s != null`, `s == null` and `Objects.requireNonNullElse(s, d)` on such a
  result; a use of the result as a plain string without a test keeps its present reading.
- The evaluator and the Go writer handle the form; the quickstart reads with 2 refusals fewer.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/jvm` types, `docs/design.md`,
  the Java reader's tests and README. Inferred.
