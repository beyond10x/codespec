---
format: aep.planning-md/3
id: epic:errors-cross-every-adapter
kind: epic
status: active
title: Errors cross every adapter
relations:
- informed_by: story:rust-writer-error-propagation
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T18:32:51Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T18:38:43Z", actor: "human:timo", revision: 4}
---
# Epic: Errors cross every adapter

## Outcome

A fallible call whose error the caller returns unchanged (`sem.error.propagate`) is read by every
reader that has an error model (Go, Rust, Java) and written by both writers, and an example runs
it under the harness: a Java method that calls a throwing method is translated to Go and to Rust
and the three run byte-equal.

## Why Now

`story:rust-writer-error-propagation` names the feature, makes the Go reader read `err != nil`
and makes the Rust writer write the form with `?`. The Rust reader lists the id in `not` (it keeps
`?` as `foreign`), the Java reader already lifts a call of a throwing method into the form but
does not claim the id, and `examples/ledger` was reshaped so that `Account.post` does not throw,
which hides the form the feature exists for.

## Out of Scope

- The Groovy reader: it refuses `throw` and claims no `type.error` (`adapters/groovy/README.md`);
  an exception model for Groovy is its own epic.
- Wrapping an error with context (`errorf` around a callee's error): stays refused.
- `comma_ok` calls in the Rust writer.
- Exceptions caught and handled in Java (`try`/`catch`).

## Done When

- `website/data/conformance/reader-go.json`, `reader-rust.json`, `reader-java.json`,
  `writer-go.json` and `writer-rust.json` list `sem.error.propagate` as claimed and `pass`.
- `examples/ledger` has a method that calls a throwing method and lets the exception propagate,
  and `task examples-ledger` is byte-equal over the original Java, the Go and the Rust.
- The Go reader reads the idiomatic spelling `if err := f(); err != nil { return …, err }` as the
  propagation form wherever hoisting the init keeps its meaning, so a Go source written as Go is
  usually written reaches the writers' `?` path.
