---
format: aep.planning-md/3
id: story:rust-reader-std-types-by-path
kind: story
status: archived
title: The Rust reader maps a std type only when its path is std's
relations:
- decomposes: epic:async-network-service-translation
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-25T01:44:35Z", actor: "human:operator", revision: 3}
---
# Story: The Rust reader maps a std type only when its path is std's

## Outcome

A type, call or method the reader maps by its std meaning (`Sender`, `Receiver`, `HashMap`,
`Mutex`, `Vec`, `String`, `Rc`) is mapped only when the `use` declarations resolve it to std (or
the prelude). A type of the same last segment from another crate is an extern
(`story:rust-reader-externs`) or a refusal, never the std form.

## Why

Names resolve by their last path segment (crate doc, `use` row). A probe field
`events: broadcast::Sender<String>` (`use tokio::sync::broadcast`) is read as `chan<- string`
with no refusal, and the Go writer writes `Events chan<- string`. A broadcast sender's `send` does
not block and returns a count; a Go send blocks. The translation is silently wrong.

## Acceptance

- A test crate with `tokio::sync::broadcast::Sender`, `tokio::sync::mpsc::Sender` and a
  `HashMap` of another crate, each imported by `use` and by full path: none is read as the std
  form; the std forms beside them still are.
- Every checked-in example reads byte-equal.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.

## Archived

Archived 2026-11-24: no remainder. `story:rust-reader-no-silent-misread` (implemented) matches
every type, call and trait of the reader's tables by the full path a path names after `use`, never
its last segment; a method is looked up on the receiver's type, which that table already matched.
On main `5f82fff97` these tests in `adapters/rust-read/tests/reader.rs` cover this story's
acceptance and pass:

- `a_type_path_maps_by_its_full_path_so_a_broadcast_sender_is_no_channel` (line 3185);
- `every_type_table_entry_is_matched_by_its_full_path_not_its_last_segment` (line 3208);
- `every_call_table_entry_is_matched_by_its_full_path_not_its_last_segment` (line 3286).

`Mutex` and `Arc`, not in the tables yet, take the same full-path rule in
`story:rust-reader-shared-state`. `story:example-async-service` depends on
`story:rust-reader-no-silent-misread` instead.
