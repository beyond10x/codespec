---
format: aep.planning-md/3
id: story:reader-check-externs
kind: story
status: draft
title: Extern calls of a non-Go reader run at the reader check
summary: expr.extern, expr.extern_method and type.extern become judgeable for the Rust, Java and C readers.
relations:
- depends_on: story:eval-handlers-traces
revision: 1
---
# Story: Extern calls of a non-Go reader run at the reader check

## Outcome

`expr.extern`, `expr.extern_method` and `type.extern` are judged for the Rust, Java and C readers.
Today step 4 writes an extern path as a Go import, so a Java, Rust or C extern cannot run and the
three features stay unclaimed by those readers.

## Acceptance

- Not written yet: needs the handlers of `story:eval-handlers-traces` and a decision on how a
  case names the handler of a non-Go extern.

## Scope

- Not scoped. No wave has taken this story.
