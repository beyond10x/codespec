---
format: aep.planning-md/3
id: story:go-reader-whole-package
kind: story
status: implemented
title: The Go reader reads every package of a repository, refusing per declaration
relations:
- decomposes: epic:whole-repo-to-syscalls
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/encode.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/ir.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/outside.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/whole_package_test.go
- confidence: cited
  path: docs/static-analysis.md
- confidence: cited
  path: scripts/classify.mjs
- confidence: cited
  path: scripts/read-repo.sh
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:52Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:52Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T02:13:46Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: The Go reader reads every package of a repository, refusing per declaration

## Outcome

`codespec read --lang go <dir>` succeeds on every non-test package of the measured library
(the 228 imported packages are read by `story:link-modules`): a declaration with no `cir/1` form is
refused with its reason, and the rest of the package is read.

## Context

- Before: every package of the library failed with "want exactly one package" or "type foreign … has no
  cir/1 form" (§ 5a).
- `adapters/go/cmd/codespec-read-go/front.go` `load`: files chosen by `go/build` `ImportDir` (build constraints, no test
  files). `adapters/go/cmd/codespec-read-go/outside.go`: `scanOutside` to a fixed point, `signatureOutside`,
  per-statement foreign with "type outside cir/1: T". A function with no Go body is refused
  "function without a Go body (assembly or linkname)".
- Unsigned and 8/16-bit integers read (`signed: false`, `bits`).
- `scripts/read-repo.sh <repo> <out>`: reads every package of `go list ./...` and prints one line
  per package; `scripts/classify.mjs <out>` groups refusals by cause. Both in the repository.

## Acceptance

- `adapters/go/cmd/codespec-read-go/` test: a package with a struct holding a `sync.Mutex` field, a variadic function, a
  body-less function and a `_test.go` file reads; each of the three is refused with its reason,
  the test file is not read, and the other functions are in the document.
- `scripts/read-repo.sh` on the measured library exits 0 for all 6 packages.
- `task check` passes.
