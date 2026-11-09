---
format: aep.planning-md/3
id: story:c-writer
kind: story
status: implemented
title: A C writer and the C runner
relations:
- decomposes: epic:c-reader-writer
scope:
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: cited
  path: adapters/c
- confidence: cited
  path: adapters/c-run
- confidence: cited
  path: crates/docs/src/support.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-09T04:18:55Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: A C writer and the C runner

## Outcome

`codespec-write-c <out dir>` (crate `adapters/c`) reads one `cir/1` module on stdin and writes C11
into the out dir: one `.c` file, one `.h` file and `codespec_rt.h`, a header-only runtime for the
`cir/1` types C lacks (string, slice, map, the error value of a `(T, error)` result). The output
builds with `cc -std=c11 -Wall -Werror` and links no library beyond libc.

Integer arithmetic of `cir/1` wraps (`sem.int.wrap`). The writer emits signed `+`, `-`, `*` and
negation through unsigned arithmetic and a conversion back, so the result is defined and equals the
Go and Rust output.

`codespec-run-c --build <dir> <written dir> <calls.json>` (crate `adapters/c-run`) compiles the
written C with a generated driver that makes the calls of `calls.json` and prints their results in
the kit's output format. It exits 0 with the output, 2 when it refuses the calls, and 1 when the
build or the program fails (the runner contract in `crates/conformance/src/conform.rs:12-20`).

## Acceptance

- `codespec conform --lang c --role writer` conforms on every feature the writer claims, and
  `task docs-generate` checks in `website/data/conformance/writer-c.json`.
- The writer claims `sem.int.wrap`, and the kit case `sem-int-wrap` (`Add(9223372036854775807, 1)`)
  passes through `codespec-run-c`. A crate test builds a copy of the writer's output with a plain
  signed `+` under `-fsanitize=undefined -fno-sanitize-recover` and shows that copy fails the case.
- A module with a construct the writer cannot express exits 1 and lists it in `refusals.json`, as
  the Go and Rust writers do.
- `examples/orders/orders.cir.json` written as C builds with `-Wall -Werror`. The goroutine stays a
  refusal.
- `codespec-write-c` and `codespec-run-c` are in the release archive (`release-build.yml`) and smoke
  tested on the kit case `op-add`.
- `c` is in `crates/docs/src/support.rs` `LANGUAGES`, and the support matrix has a C writer row.
- `task check` green.

## Out of Scope

- The reader (`story:c-reader-clang`).
- The harness on a C implementation of an example (`examples/*/harness.toml` with C): a later
  story.

## Shared surface and merge order

The operator asked (2026-11-09) for `story:c-reader-clang`, `story:c-writer`,
`story:builtin-defect-checks` and `story:use-cases-page` to run in one wave. They share these
files, each by appending one entry:

- `Cargo.toml` `[workspace] members` and `Cargo.lock`: `adapters/c-read`, `adapters/c`,
  `adapters/c-run`, `crates/check`.
- `.github/workflows/check.yml`: one test-matrix line per new crate.
- `Taskfile.yml`: the `adapters` task and the manifest loops (C stories only).
- `crates/docs/src/main.rs` `REPORTS` (C writer only), `crates/docs/src/support.rs` `LANGUAGES` and the generated
  `website/data/support-matrix.json` (C stories only).
- `crates/cli/src/lib.rs` `Cmd` (`story:builtin-defect-checks` only adds `check`; the C stories add
  no command).

Each implementor appends its own entries and touches no other story's. The coordinator merges in the
order `story:c-writer`, `story:c-reader-clang`, `story:builtin-defect-checks`,
`story:use-cases-page`, resolves the append conflicts in those files by keeping both entries, and
runs `task check` after each merge.

Generated files (`website/docs/reference/crates.md`, `website/docs/reference/cli.md`,
`website/data/support-matrix.json`, `website/data/conformance/*.json`) are never merged by hand:
the coordinator takes either side and runs `task docs-generate` after each merge.
`crates/docs/src/support.rs` `LANGUAGES` gains `c` in `story:c-writer` only. `story:c-reader-clang`
does not edit `LANGUAGES` or `REPORTS`.
