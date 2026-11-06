---
format: aep.planning-md/3
id: story:codespec-conform
kind: story
status: implemented
title: codespec conform
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:conformance-cases
- depends_on: story:runners-and-manifests
scope:
- confidence: cited
  path: conformance/reader
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/conformance
- confidence: cited
  path: crates/docs/src/main.rs
- confidence: cited
  path: crates/docs/src/status.rs
- confidence: cited
  path: website/data/conformance
- confidence: cited
  path: website/docs/reference/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:54Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: codespec conform

## Outcome

`codespec conform --lang <lang> --role reader|writer --out <report.json> [--kit <dir>] [--corpus <dir>]` runs the kit
against an adapter and writes `codespec-conformance-report/1` with the verdicts of
`docs/conformance.md` § 6–7; `codespec conform --check <report>` re-runs and compares.

## Acceptance

- Manifests are expanded against `conformance/kit.json`: a trailing `*` per family, `not`
  subtracting; a test expands `["stmt.*"]` with `not: ["stmt.send"]` to the 15 other statement ids.
- Writers: the Go and Rust writers' reports say `conforms: true` for their manifests.
- A copy of the Rust writer whose integer addition saturates instead of wrapping gets
  `conforms: false` naming `sem.int.wrap` and the first differing line.
- With stub writers, one test each: a claimed feature refused is `fail`; an unclaimed feature
  written wrong is `fail`; a writer that writes a module holding an unknown statement kind, or
  writes twice with different bytes, gets `conforms: false` naming the boundary case.
- Readers: the reader corpus for Go and Rust (`conformance/reader/go/<case>/`,
  `conformance/reader/rust/<case>/`: source, `calls.json`, one per feature the reader claims) is
  added; a case's subject feature found only inside `foreign` fails step 3 of § 6, with a test; the
  Go and Rust readers' reports say `conforms: true`.
- All four reports are generated into `website/data/conformance/` by `codespec-docs` and
  drift-checked by `task docs-check`; `task check` runs them.
- The Status page gains one capability row per adapter whose evidence is its report.
- `codespec conform --check <report>` exits 0 on a current report and 1, naming the feature, after
  a case's `expected.txt` is changed.

## Scope

- `crates/conformance/` (the `conform` engine), `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`,
  `website/docs/reference/cli.md` (regenerated).
- `conformance/reader/` (new).
- `website/data/conformance/` (new), `crates/docs/src/main.rs` (the `outputs` list),
  `crates/docs/src/status.rs`.
- `Taskfile.yml`: the coordinator's.
