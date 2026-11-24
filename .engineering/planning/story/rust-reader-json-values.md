---
format: aep.planning-md/3
id: story:rust-reader-json-values
kind: story
status: draft
title: The Rust reader reads serde_json::Value and json!
summary: Value as a type, json!, indexing and the as_* accessors are read into cir/1 instead of refusing every function that touches them.
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-externs
revision: 2
---
# Story: The Rust reader reads `serde_json::Value` and `json!`

## Outcome

A parameter, field or result of type `serde_json::Value`, the `json!` macro, indexing
(`v["k"]`, `v[0]`) and the accessors `as_i64`, `as_str`, `as_bool`, `is_null` with
`unwrap_or`, and `to_string` are read into `cir/1` (an extern type with its methods, mapped by
the Go writer to its JSON form), where today each function touching them is refused.

## Why

Consumer report on 0.12.0 from an async network service: "values of … JSON" among its top
refusals. Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
use serde_json::{json, Value};
pub fn size_of(v: Value) -> i64 { v["size"].as_i64().unwrap_or(0) }
pub fn make(size: i64) -> Value { json!({ "size": size }) }
pub fn encode(v: &Value) -> String { v.to_string() }
```

`read 0 of 3 functions`; each refused at its signature: "type `Value` (`serde_json::Value`) is
outside cir/1: the reader maps a path by the full path it names".

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with every
  body in `cir/1`, red on main.
- A `json!` with a form the reader cannot map (a spread, a computed key) stays refused naming it.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/reader.rs`, the Go writer's extern map
  (`story:go-writer-extern-map`), `CHANGELOG.md`. Reproducer: `target/repro/json/`.
