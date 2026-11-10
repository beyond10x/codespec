---
format: aep.planning-md/3
id: story:cir1-fields-of-imported-types
kind: story
status: draft
title: 'cir/1: fields of a struct another module of the same read declares'
revision: 1
---
## Outcome

A function in one module of a project read can read and write a field of a struct that another
module of the same read declares, and the document says which module's struct it is. Today a
type of another module is an extern type, which is opaque: `cir/1` has no form for its fields, so
the Java reader refuses the access at its line (`adapters/java/src/codespec/read/Reader.java`
near line 3860) and the Go reader refuses the same form (`adapters/go/cmd/codespec-read-go/front.go`
near line 1046, "the extern type is opaque").

## Why

Found by the adversary pass on `story:reader-java-quarkus-refusals`
(`review-result:adversary-reader-java-quarkus-pass-1`, F5). Before the fix the Java reader mapped
such a field with no check that the other module declares it. After the fix the Quarkus
quickstart used as the measure has 2 refusals of this kind (a resource reading `name` of an
entity in another package). A project whose entities and resources live in different packages
has one per access.

## Acceptance

- `cir/1` has a documented form by which a module names a struct type of another module of the
  same read and reads or writes its fields; strict reading refuses the form when the named
  module or field is absent from the set of documents it is given.
- The Java reader and the Go reader read a cross-package field access in a project read without
  a refusal, each with a red test first; the evaluator runs it; the writers either write it or
  refuse it at its position.
- The Quarkus quickstart read has 0 refusals whose reason names a field of a class of another
  package.

## Not in scope

A field of a type outside the read (a library class) stays refused.
