---
format: aep.planning-md/3
id: story:edit-spans-format
kind: story
status: proposed
title: codespec-spans/1 and codespec read --spans
summary: The span document beside cir/1, its strict reading, and read --spans / --overlay in the CLI
relations:
- decomposes: epic:code-edit-language
- supersedes: story:reader-function-spans
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/edit.rs
- confidence: inferred
  path: crates/cli/tests/fixtures
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/src/spans.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:47Z", actor: "human:timo", revision: 3}
---
# Story: codespec-spans/1 and `codespec read --spans`

## Outcome

`codespec-spans/1` is defined in `crates/model/src/spans.rs` (beside `manifest.rs`, not part of
`cir/1`) with strict reading against its `cir/1` document and the source files, as
`docs/code-edit-language.md` § 6.2 states it: format, no unknown field, `cir_sha256`, file SHA-256
and length, spans inside their file on UTF-8 boundaries, `line`/`end_line` consistent with the
bytes, `text` equal to the bytes, every `at` naming a node of the document, no overlapping `name`
spans, and completeness (`decl` for every function and type, `node` for every statement, `name`
for every node of a kind wave 1 renames, or an `unspanned` entry). Canonical encoding, sorted by
`at` then `role`.

`codespec read --spans <out> --lang <l> <dir>` runs the reader with `--spans`, removes the
`span`, `name_span`, `span_files`, `unspanned` and `refs` members from its output, computes each
`at` from the JSON path where it found them, strict-reads the remaining `cir/1` and the span
document, and writes both. `codespec read --overlay <file>` passes the overlay to the reader.
The adapter protocol (`--spans`, `--overlay`, the `spans` mark in `--describe`) is written in
`docs/design.md` § 2.

## Done When

- `crates/model/tests/model.rs` (a module of the existing target, no new test binary): one
  refusing test per strict-reading rule above, each naming the rule in its message; a valid
  document reads and re-encodes byte-identically.
- `crates/cli/tests/edit.rs` (new; the one test target for `read --spans` and `edit`, in the existing `test b10x-codespec-cli` part): with a fixture reader (a POSIX sh script printing a fixed spanned module), the
  `cir/1` written with `--spans` is byte-identical to the one written without, and the span
  document equals a checked-in file; a fixture whose `text` differs from the file's bytes, and one
  missing a `name` span with no `unspanned` entry, each exit non-zero naming the `at`.
- `crates/cli/tests/edit.rs`: a test maps every `at` below `funcs.<i>.body` to the cq node id the fact base gives the same node,
  over `examples/series/series.read.cir.json` (every statement and expression).
- `task docs-generate` run; `website/docs/reference/cli.md` current; `CHANGELOG.md` entry.
- No test over 5 s; no Python.

## Scope

- Own: `crates/model/src/spans.rs` (new), `crates/model/src/lib.rs` (the `mod` line),
  `crates/model/tests/model.rs`, `crates/cli/src/lib.rs` and `crates/cli/src/main.rs` (`Read`
  flags), `crates/cli/tests/edit.rs` (new) and `crates/cli/tests/fixtures/`, `docs/design.md` § 2, `website/docs/reference/cli.md` (generated),
  `CHANGELOG.md`.
- Same files as `story:edit-cli-verified-apply` (`crates/cli`, `CHANGELOG.md`), which runs after it.
