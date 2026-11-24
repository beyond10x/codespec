---
format: aep.planning-md/3
id: story:edit-spans-format
kind: story
status: proposed
title: codespec-spans/1, crates/edit and codespec read --spans
summary: The span document beside cir/1 in a new crates/edit, its strict reading, the at/node-id mapping, read --spans
relations:
- decomposes: epic:code-edit-language
- supersedes: story:reader-function-spans
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/cli/Cargo.toml
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/edit
- confidence: inferred
  path: crates/cli/tests/fixtures/edit
- confidence: inferred
  path: crates/cli/tests/query.rs
- confidence: inferred
  path: crates/edit
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:47Z", actor: "human:timo", revision: 3}
---
# Story: codespec-spans/1, crates/edit and `codespec read --spans`

## Outcome

A new crate `crates/edit` (`b10x-codespec-edit`, library only) starts with
`src/spans.rs`: the `codespec-spans/1` document of `docs/code-edit-language.md` § 6.2 and its
strict reading against its `cir/1` document and the source files (format, no unknown field,
`cir_sha256`, each file's SHA-256 and length, spans inside their file on UTF-8 boundaries, `line`
and `end_line` consistent with the bytes, `text` equal to the bytes, every `at` naming a node of a
kind its role allows, no overlapping `name` spans, and completeness: a `decl` span for every
function and a `name` span for every node of a kind wave 1 renames, or an `unspanned` entry).
Canonical encoding, sorted by `at` then `role`. SHA-256 comes from the workspace's `sha2`.

`src/ids.rs` maps an `at` path below `funcs.<i>.body` to its cq node id and back.

`codespec read --spans <out> --lang <l> <dir>` runs the reader with `--spans`, takes its JSON
object `{cir, files, spans, unspanned, refs}` (§ 6.3), strict-reads the `cir/1`, adds `module` and
`cir_sha256`, strict-reads the span document and writes both. A reader that fails on `--spans` or
prints a bare module is refused: "the <lang> reader gives no spans". The protocol is written in
`docs/design.md` § 2.

## Done When

- `crates/edit/tests/main.rs` (the crate's one test target): one refusing test per strict-reading
  rule above, each naming the rule and the `at` in its message; a valid document reads and
  re-encodes byte-identically.
- Same target: every `at` below `funcs.<i>.body` of `examples/series/series.read.cir.json` maps to
  the node id the cq fact base gives that node, and every node id of that fact base maps back to
  its `at` (`b10x-codespec-query` as a dev-dependency).
- `crates/cli/tests/edit/mod.rs`, a module of the existing `crates/cli/tests/query.rs` target (one
  `mod edit;` line; no new test binary), with fixture readers that are POSIX sh scripts printing
  fixed output: the `cir/1` written with `--spans` is byte-identical to the one written without, and
  the span document equals a checked-in file; a fixture whose `text` differs from the file's bytes,
  and one missing a `name` span with no `unspanned` entry, each exit non-zero naming the `at`; a
  fixture that prints a bare module exits non-zero with "gives no spans".
- `scripts/check-parts.txt` names `b10x-codespec-edit` in a `test` part; `task check-parts` passes.
- `task docs-generate` run: `website/docs/reference/cli.md` and `crates.md` (the new crate's row)
  current; `CHANGELOG.md` entry.
- No test over 5 s; no Python.

## Critic findings answered

- `--overlay` untested (acceptance): removed. Re-reading uses a cache copy
  (`story:edit-slice-rust-rename`, § 7.3 of the design), so no reader gets an overlay flag.
- `docs/design.md` text untested (acceptance): it is documentation and is reviewed in the pull
  request; the behaviour it describes is tested by the CLI cases above.
- A `spans` mark in `--describe` needs an `AdapterManifest` change (parallel safety, design): no mark.
  A reader without spans is detected by its output, tested above; `crates/model` is not changed.
- `sha2` and `Cargo.lock` (parallel safety): the format lives in `crates/edit`, which takes
  `sha2.workspace`; `Cargo.toml` and `Cargo.lock` are in scope.
- The new crate's row in `website/docs/reference/crates.md` (parallel safety): in scope here.
- A new test binary `crates/cli/tests/edit.rs` (parallel safety): replaced by a module of the
  existing `query.rs` target.
- Nobody owns the `at` ↔ node id mapping (design): this story, `src/ids.rs`.
- `node` spans for every statement (scope): dropped from wave 1; the format stays open for the role.

## Scope

- Own: `crates/edit/` (new: `Cargo.toml`, `src/lib.rs`, `src/spans.rs`, `src/ids.rs`,
  `tests/main.rs`, `tests/fixtures/`), the workspace `Cargo.toml` (members), `Cargo.lock`,
  `scripts/check-parts.txt`, `crates/cli/Cargo.toml`, `crates/cli/src/lib.rs` and
  `crates/cli/src/main.rs` (`Read` flags), `crates/cli/tests/query.rs` (the `mod edit;` line),
  `crates/cli/tests/edit/` (new), `crates/cli/tests/fixtures/edit/`, `docs/design.md` § 2,
  `website/docs/reference/cli.md` and `crates.md` (generated), `CHANGELOG.md`.
- First story of the wave; every other wave-1 story runs after it.
