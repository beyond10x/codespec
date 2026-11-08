---
format: aep.planning-md/3
id: story:syscall-barrier
kind: story
status: implemented
title: The effect report reaches the syscall barrier and names the syscalls
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:link-modules
- depends_on: story:foreign-calls
- depends_on: story:dispatch-over-set
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/encode.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/ir.go
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/tests/barrier.rs
- confidence: cited
  path: crates/cli/tests/fixtures/readfile/
- confidence: cited
  path: crates/effects/
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: docs/static-analysis.md
- confidence: cited
  path: schemas/cir-1.schema.json
- confidence: cited
  path: summaries/go-linux-amd64.barrier.json
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:30Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
# Story: The effect report reaches the syscall barrier and names the syscalls

## Outcome

Reading the Go standard library down to the barrier, `codespec effects` reports per function the
set of syscalls it may make (`syscalls: ["connect", "openat", "sendto", …]`) and maps each to
effects, and each effect and syscall carries the positions of the calls in that function that
lead to it (`docs/static-analysis.md` § 5c, "where does a side effect happen"); the hand-written part is only the table at the barrier.

## Context

- The barrier in Go 1.27.1 (`docs/static-analysis.md` § 5): `syscall.Syscall6` →
  `RawSyscall6` → `internal/runtime/syscall/linux.Syscall6`, no Go body, `asm_linux_amd64.s`;
  callers pass `SYS_*` constants (`zsysnum_linux_amd64.go`, 303 names).
- `summaries/go-linux-amd64.barrier.json`, format `codespec-barrier/1`: the body-less functions
  that take a syscall number and which argument holds it; the table syscall name → effects; `read`,
  `write`, `close` and siblings map to a new effect `io` (`docs/static-analysis.md` § 5: the
  descriptor's origin decides fs, net or stdin).
- `//go:linkname`: the reader records the directive; the link step resolves a body-less function
  in one package to the function in another that is linknamed to it (`internal/poll.runtime_pollWait`
  → `runtime.poll_runtime_pollWait`).
- The `runtime` package's own syscalls (`futex`, `mmap`, scheduler) are reported only where a
  function reaches them by an explicit call.

## Acceptance

- `crates/effects/tests/effects.rs`: a set with `syscall.openat` calling the barrier with
  `SYS_OPENAT` reports `syscalls: ["openat"]` and the table's effects for its caller.
- `crates/cli/tests/barrier.rs`: runs `scripts/read-repo.sh --deps` on the fixture module
  `crates/cli/tests/fixtures/readfile/` (one function calling `os.ReadFile`), then `codespec
  effects` over the documents with the barrier; that function's `syscalls` include `openat` and
  `read`, its effects include `io`, and its `openat` entry carries the position of the
  `os.ReadFile` call.
- `crates/effects/tests/effects.rs`: a body-less function linknamed to a function of another
  module of the set reports that function's syscalls; `read` maps to `io`.
- `task check` passes.
