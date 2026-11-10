---
format: aep.planning-md/3
id: story:rust-reader-union-field-names
kind: story
status: draft
title: The Rust reader refuses a union variant whose fields share one exported name
relations:
- decomposes: epic:reader-coverage
revision: 1
---
# Story: the Rust reader refuses a union variant whose fields share one exported name

## Outcome

An enum variant with fields `value_2` and `value2` is refused at its line, as the struct path
refuses it (`adapters/rust-read/src/lib.rs` near 2051), instead of reading to a union variant that
declares `Value2` twice.

## Why

Found by adversary pass 1 on `story:reader-rust-decl-union`, pre-existing: `lib.rs` near 2453
exports union field names with no duplicate check, and `codespec validate` accepts the module.

## Acceptance

- A reader test with such a variant gets a refusal at the variant's line and no union with a
  repeated field name.
- `codespec validate` refuses a union variant that declares one field name twice.

