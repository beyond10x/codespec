---
format: aep.planning-md/3
id: story:read-go-newer-toolchain
kind: story
status: draft
title: The Go reader reads a tree on a newer Go, or refuses by name
summary: a package importing math/rand/v2 reads under the newest Go; an older reader names the toolchain gap
relations:
- decomposes: epic:release-diff-go-field-gaps
revision: 1
---
# Story: The Go reader reads a tree on a newer Go, or refuses by name

## Outcome

The release's `codespec-read-go` is built with the Go the module pins (`adapters/go/go.mod:3`,
`go 1.23`; `release-build.yml:39` takes `go-version-file`), go1.23.12 for 0.14.0. Its `go/types`
then type-checks the standard library of the host's Go. Under go1.27 on the host it fails on any
package importing `math/rand/v2` with "method must have no type parameters": 32 of 49 packages of
a consumer's Go module failed. Running the module's own older toolchain works around it; a tree
whose `go.mod` asks for go1.27 or later still fails.

The change:

- The release builds `codespec-read-go` with the newest stable Go (a `toolchain` line in `adapters/go/go.mod`, or the release workflow's `go-version`), and the version it was built with is in `reader.version` output (`codespec-read-go --version`).
- When the host's Go (`go env GOVERSION`) is newer than the Go the reader was built with, the reader refuses before reading, naming both versions and the way out (a reader built with that Go, or `GOTOOLCHAIN=<older>`), instead of failing per package with a type error.

## Acceptance

- A synthetic Go module whose package imports `math/rand/v2` reads with no refusal under the newest Go the gate has.
- With a host Go newer than the reader's build Go, the reader exits non-zero with one line naming both versions, and reads nothing.
- `codespec-read-go --version` prints the Go it was built with.

## Reported by

A consumer moving its code facts onto the 0.14.0 release (gap G6, relayed by the host operator,
2026-11-26).
