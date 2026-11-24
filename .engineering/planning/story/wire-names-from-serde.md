---
format: aep.planning-md/3
id: story:wire-names-from-serde
kind: story
status: draft
title: Wire names of serde-derived types reach cir/1 and the Go writer
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: Wire names of serde-derived types reach cir/1 and the Go writer

## Outcome

The Rust reader records, for a type with `#[derive(Serialize)]` or `#[derive(Deserialize)]`,
each field's and variant's wire name after `rename`, `rename_all`, the enum representation
(`tag`, `content`, untagged), `default`, `skip_serializing_if` and `skip`, in an additive
`cir/1` record; an attribute it does not map refuses the type naming the attribute. The Go
writer writes the record as `json:"…"` struct tags (with `omitempty`) and a marshal method where
a tag cannot express it (an adjacently tagged enum), or refuses naming it.

## Why

Release 0.12.0 drops derives and serde attributes without a refusal: a probe struct with
`#[serde(rename_all = "camelCase")]` and a field `#[serde(rename = "type")] kind` is read as
fields `ItemID` and `Kind` with no trace of `itemId` or `type`. The Go output would encode a
different JSON body: a silent change of the service's interface, against the rule that nothing
is dropped or approximated.

## Acceptance

- Until the record exists, a serde attribute the reader does not map is a refusal at its line
  (first commit of the story), with a test.
- A test crate whose types use each listed attribute: encoded by the Rust source with
  `serde_json` and by the Go output with `encoding/json`, the bytes are equal for seeded values;
  decoding each other's output gives the same values.
- `schemas/cir-1.schema.json` regenerated; checked-in documents read unchanged.

## Scope

- `crates/model/src/lib.rs`, `schemas/`, `adapters/rust-read/src/`, `adapters/go/`,
  `CHANGELOG.md`. Inferred.
