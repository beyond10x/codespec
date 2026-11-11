---
format: aep.planning-md/3
id: story:query-listed-signatures-and-inits
kind: story
status: draft
title: codespec query holds the signatures of listed calls and the module initialisers' calls
summary: The fact base drops the sig of listed value, ref and lambda records, keeps interface method signatures only for implements, and never reads Module.inits
revision: 2
---
# Story: codespec query holds the signatures of listed calls and the module initialisers' calls

## Outcome

A cq rule can read every signature and every call a `cir/1` document states:

1. a listed function-value call (`CallRef::Value`) has its `sig` in the fact base;
2. a listed function reference (`CallRef::Ref`) and a listed function literal (`CallRef::Lambda`)
   have theirs;
3. the methods and signatures a listed interface call carries (`CallRef::Iface.methods`) are a
   relation, not only an input to `implements`;
4. the calls and function references of `Module.inits` (the initialisers of the module's
   variables) are `call` rows, so `calls`, `reaches` and a negation over them see them.

## Context

The document holds all four; the fact extraction drops them. Read at tag 0.7.0; `crates/query`
is unchanged from 0.7.0 to `main` at 0.8.0.

- The Go reader records a function-value call with its signature,
  `adapters/go/cmd/codespec-read-go/calls.go:192` and `:395`, and writes it as `sig`,
  `adapters/go/cmd/codespec-read-go/encode.go:464-465`. The model keeps it:
  `crates/model/src/lib.rs:677`, `Value { sig, pos }`; `Ref` and `Lambda` carry `sig` too
  (`lib.rs:680-695`).
- `crates/query/src/facts.rs:1829` turns `CallRef::Value { .. }` into a `call` row with kind
  `value` and empty path, type and name: the signature is gone. `CallRef::Ref { path, recv, name,
  .. }` (`facts.rs:1830-1832`) and `CallRef::Lambda { calls, .. }` (`facts.rs:1833-1836`) drop
  theirs the same way. A read body's `call_value` keeps its type through `expr`, so only listed
  calls are affected: every call of an opaque function and every call of a foreign statement.
- `facts.rs:1815-1826` stores a listed interface call's methods in `ifaces`, which feeds
  `implements` only; no relation holds a method name with its signature for a call-list
  interface (`method_set` holds the method sets the reader listed for types).
- The Go reader writes the initialisers' calls as `inits` (`encode.go:80`); the model reads them
  (`crates/model/src/lib.rs:86-89`, `Module.inits: Vec<CallRef>`). No file of `crates/query/src`
  mentions `inits`, so a function used only from a variable initialiser has no caller and no
  reference in the fact base.

## Acceptance

- `codespec query '?- call{kind: "value", listed: true, type: T}.'` over a document whose opaque
  function calls a function value prints the signature the document states, not an empty field.
  The same holds for a listed `ref` and a listed lambda.
- A relation (name to be chosen) holds, for each interface a call list names, each method's name
  and signature; `--schema` describes it.
- A function referenced only in `Module.inits` has a `call` row of kind `ref` whose `func` is the
  module's initialiser, and a call made only there has a `call` row; a kit case shows both.
- `--schema` text for `call` says what `type` holds for each kind.
- Every existing query gives the same rows on documents without listed value calls, refs,
  lambdas or initialiser calls.

## Out of scope

Following a function value to the functions it may run (a points-to or signature match) is a
rule's work once the signature is a fact.
