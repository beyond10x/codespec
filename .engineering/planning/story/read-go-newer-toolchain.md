---
format: aep.planning-md/3
id: story:read-go-newer-toolchain
kind: story
status: implemented
title: The Go reader reads a tree on a newer Go, or refuses by name
summary: a package importing math/rand/v2 reads under the newest Go; an older reader names the toolchain gap
relations:
- decomposes: epic:release-diff-go-field-gaps
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/README.md
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/encode.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/goroot.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/main.go
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/testdata
- confidence: cited
  path: adapters/go/go.mod
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T15:11:24Z", actor: "human:operator", revision: 15}
- {from: "proposed", to: "active", at: "2026-11-26T15:11:24Z", actor: "human:operator", revision: 16}
- {from: "active", to: "implemented", at: "2026-11-26T18:43:27Z", actor: "human:operator", revision: 17, decided_on: {"recorded":{"test_result":1}}}
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

## Found while scoping

- The local gate and the forge build the reader with different Go today: the host's `go` is go1.27.1 with `GOTOOLCHAIN=auto`, while `setup-go` in `check.yml:103-105` and `release-build.yml:37-39,133-135` installs the newest 1.23.x from `go.mod`. The three `setup-go` steps name one `go-version` explicitly (unless a forge run proves that `setup-go` reads a `toolchain` line), so the gate and the release build with the same Go.
- The build Go goes into `codespec-read-go --version` output only: `reader` in cir/1 is `deny_unknown_fields` (`crates/model/src/lib.rs:822-829`) and `reader.version` is the workspace version.
- The host's GOVERSION carries a suffix (`go1.27.1-X:nodwarf5`); it is cut before comparing. The refusal compares minor versions, not patch releases.
- A portable refusal test puts a stub `go` first on PATH that prints a newer GOVERSION.
- The check runs before reading and before `--spans`; `--describe` reads no package and is left alone.
