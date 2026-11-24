---
format: aep.planning-md/3
id: story:sealed-translation-run
kind: story
status: draft
title: A translation runs sealed from one release archive, offline
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:example-async-service
revision: 1
---
# Story: A translation runs sealed from one release archive, offline

## Outcome

A guide page and a check: with one release archive (`codespec-<tag>-<target>.tar.gz`, verified
against `SHA256SUMS`), the Rust toolchain `rust-toolchain.toml` pins and Go, and a vendored Go
module directory for the written program's mapped externs, the steps read, pass, write, build
and harness run with no network. The page lists, per step, the program, the toolchain and the
files it needs.

## Why

The steps need different things: `codespec-read-rust` parses with `syn` and needs no rustc,
cargo or crate dependency (probes read with no dependency resolved); `codespec-write-go` prints
Go with no toolchain; building and vetting the Go output, `codespec-run-go` (`go build`) and a
harness's Go driver need Go; the Rust original's driver needs cargo with its dependencies
vendored (`codespec-run-rust` builds `--offline`). Nothing in the repository states this, or
runs it sealed.

## Acceptance

- `website/docs/guides/` page with the table per step; generated or checked like the other
  guides.
- A gate part or task runs `story:example-async-service`'s steps from the unpacked release
  layout with the network unshared and passes.

## Scope

- `website/docs/guides/`, `Taskfile.yml`, `CHANGELOG.md`. Inferred.
