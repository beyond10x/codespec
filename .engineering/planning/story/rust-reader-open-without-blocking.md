---
format: aep.planning-md/3
id: story:rust-reader-open-without-blocking
kind: story
status: draft
title: The Rust reader opens a crate file without blocking on what it finds there
relations:
- decomposes: epic:codebase-transformations
revision: 1
---
# Story: the Rust reader opens a crate file without blocking on what it finds there

## Outcome

Someone who points the Rust reader at a crate they did not write gets a refusal or a reading in
bounded time, even when a file of the crate is swapped for a named pipe or a device while the
reader runs.

## Context

`files::regular_inside` (`adapters/rust-read/src/files.rs:112`) checks with `std::fs::metadata`
that the crate root, each module file and `Cargo.toml` (`adapters/rust-read/src/lib.rs:226`) is a
regular file inside the crate, and `read_bounded` (`adapters/rust-read/src/lib.rs:431`) then opens
the path again with `File::open`. A file replaced by a named pipe between the two calls is opened
as a pipe, and the open blocks until a writer appears. Found in review of pull request 21 (round 3),
left out of that pull request on purpose.

## Acceptance

- The reader opens each file once, without blocking (`O_NONBLOCK`, or `O_PATH` then a check), and
  checks the opened handle's metadata: a handle that is not a regular file is refused before any
  read, and the read uses that same handle.
- A test that replaces a module file with a named pipe between the containment check and the open
  (a hook or a seam in the reader, no timing race) gets a refusal within a bounded time.
- The containment check applies to the path the handle was opened from, as before.

## Out of Scope

- Windows behaviour beyond refusing what is not a regular file.
- Files that grow while read: the byte limit of `read_bounded` already bounds them.

## Ambiguities

None found.

## Open Questions

None.
