---
format: aep.planning-md/3
id: story:node-ids-one-function
kind: story
status: draft
title: Node ids come from one function
summary: The query crate and the evaluator share one id and position function for cir/1 nodes.
revision: 1
---
# Story: Node ids come from one function

## Outcome

The query crate and the evaluator name a node of a `cir/1` document with the same id and the
same position text, because both call one function.

## Why

Each crate computes ids on its own (`crates/query/src/facts.rs`, `crates/eval/src/id.rs`). After
the first reviews they agree on statements, expressions and struct literal fields, and differ in
three places: the query crate names the call list of a foreign statement (`<stmt>.calls.<k>`) and
of an opaque function (`<func>#calls.<k>`) and the evaluator names neither; the evaluator names
nodes in a module variable's initializer (`<var>#init.…`) and the query crate names none; the
query crate's positions are `<module path>/<file>:<line>` and the evaluator's `<file>:<line>`.
A finding of a query and a stop of the evaluator cannot be joined on a node while they differ.

## Acceptance

- One function, in a crate both depend on, gives the id and the position text of every node;
  `docs/query-language.md` § node ids states the three cases above.
- A test walks every checked-in `case.cir.json` and compares the ids both crates report.

## Scope

- Own: `crates/query/src/facts.rs`, `crates/eval/src/id.rs`, and the crate that takes the shared
  function. Inferred.
