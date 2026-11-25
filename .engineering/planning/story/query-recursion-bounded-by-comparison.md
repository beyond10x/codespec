---
format: aep.planning-md/3
id: story:query-recursion-bounded-by-comparison
kind: story
status: active
title: cq accepts a recursion whose counter is bounded by a comparison with a finite value
summary: 'Defect since 0.11.0: a depth-limited recursion (D < M, D2 = D + 1) is refused at compile time on any documents.'
relations:
- decomposes: epic:codebase-transformations
scope:
- confidence: inferred
  path: crates/query/src/engine.rs
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T01:41:52Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-25T01:41:52Z", actor: "human:operator", revision: 5}
---
# Story: cq accepts a recursion whose counter is bounded by a comparison with a finite value

## Outcome

`codespec query` runs a recursive rule that makes a new value by arithmetic from a derived value
when the same rule bounds that value, or the value it was made from, by a comparison with a
finite value (`D < M` with `M` from a relation outside the recursion, or a literal). Such a rule
derives finitely many values, so the finiteness check (`finite` in `crates/query/src/engine.rs`)
accepts it; a recursion without such a bound stays refused with today's message.

## Why

Defect, not a Java one. Reported by the consumer of a Java service: a lock-reach rule of a downstream
rule library exits 2 on its Java documents with "reach: a rule of a recursion puts
a new value into its head, made by arithmetic or a builtin from a value the recursion derived: it
could derive without end". The check runs at compile time on the rule alone: the documents never
matter. It refuses the same rule on a Go document and on the Java ones, so it is no Java shape
making a relation grow. The rule terminates: its depth counter is bounded by `D < M`, `M` a fact.
The check came with `91d0265ee` (0.11.0) and counts only a relation, a names list, a path or an
aggregate as a bound, not a comparison. Every rule library written before 0.11.0 with a
depth-limited recursion is refused since.

## Reproducer

`depth.cq`, run on main `5f82fff97`:

```text
depth_limit(4).
edge(S, G) :- calls{site: S, target: G}.
reach(F, G, 1) :- call{func: F, site: S}, edge(S, G).
reach(F, H, D2) :- reach(F, G, D), depth_limit(M), D < M, call{func: G, site: Y}, edge(Y, H), D2 = D + 1.
?- reach(F, G, D).
```

## Observed

`codespec query --file depth.cq` on Java documents read by `codespec-read-java`, and on
`examples/orders/orders.read.cir.json` (Go): both exit 2 with

```text
codespec: reach: a rule of a recursion puts a new value into its head, made by arithmetic or a builtin from a value the recursion derived: it could derive without end; make the value from facts outside the recursion, or bound it by a relation of the fact base
```

The same rule with the bound as a relation (`depth(1). … depth(4).` and `D2 = D + 1, depth(D2)`)
runs: exit 1 with one row on the Java documents. The downstream rule text gives the same exit 2
on the Go document.

## Done When

- A failing test in `crates/query/tests/query.rs` first: the rule above over a fixture with a call
  chain longer than the bound returns exactly the rows of depth 1 to 4, and the same rule without
  `D < M` is still refused with today's message.
- A comparison counts as a bound only against a finite value and only in the direction that
  bounds (`D < M`, `D <= M`, `M > D`); `D > M` alone stays refused (a test).
- The cq reference (`website/docs`, the `codespec query` section) states the accepted bound.
- `CHANGELOG.md` names the change; `task check` green.

## Scope

- Own: `crates/query/src/engine.rs` (`finite`), `crates/query/tests/query.rs`, the cq page under
  `website/docs`, `CHANGELOG.md`. Inferred.
