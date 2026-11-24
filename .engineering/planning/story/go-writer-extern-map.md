---
format: aep.planning-md/3
id: story:go-writer-extern-map
kind: story
status: draft
title: The Go writer maps externs of another language to Go packages through a reviewed file
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:extern-ports
revision: 1
---
# Story: The Go writer maps externs of another language to Go packages through a reviewed file

## Outcome

`codespec write --lang go --extern-map FILE` reads a reviewed mapping, one entry per source
extern (path, name, signature) to a Go import path and function or method, with argument and
result adaptations from a closed list (reorder, drop a context, wrap an error). The writer
writes each mapped extern as that Go call and refuses an unmapped one naming it, or hands it to
a port when `story:extern-ports` is in use. A mapping for the async runtime's time and channel
calls, `serde_json` and the observability vocabulary ships with the writer.

## Why

The Go writer writes an extern's path verbatim as a Go import path
(`adapters/go/cmd/codespec-write-go/write.go`, case `extern`). That is right for a Go source
and wrong for any other: a Rust path (`tokio::time::sleep`, a bus client's `request`, a
key-value client's `set_ex`) is no Go package. `story:reader-check-externs` names the same gap
for the reader check.

## Acceptance

- A writer test per adaptation kind; an unmapped extern is refused naming its path.
- The shipped mapping covers the externs `story:example-async-service` calls; its Go output
  builds with only the standard library and the ports.
- A mapping entry whose Go signature does not type-check against the extern's is refused at
  load, naming the entry.

## Scope

- `adapters/go/cmd/codespec-write-go/`, `crates/cli/src/lib.rs`, a mapping file under
  `adapters/go/`, `CHANGELOG.md`. Inferred.
