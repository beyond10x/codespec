---
format: aep.planning-md/3
id: story:c-interfaces-generics-closures
kind: story
status: draft
title: Interfaces, type parameters and closures in C, written and read
summary: A fixed C form for the 8 features codespec-write-c refuses, written and read back.
revision: 1
---
# Story: Interfaces, type parameters and closures in C, written and read

## Outcome

`codespec-write-c` writes, and `codespec-read-c` reads back, a fixed C form for the 8 features the
C writer refuses today: `decl.interface`, `decl.type_params`, `expr.iface_call`,
`expr.instantiate`, `expr.lambda`, `expr.spread`, `expr.to_iface`, `type.param`
(`website/data/conformance/writer-c.json`).

## Acceptance

- Not written yet: needs a design of the C forms (a table of function pointers, monomorphised
  copies, a function pointer with an environment).

## Scope

- Not scoped. No wave has taken this story.
