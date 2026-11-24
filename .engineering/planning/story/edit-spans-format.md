---
format: aep.planning-md/3
id: story:edit-spans-format
kind: story
status: active
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:45Z", actor: "human:timo", revision: 8}
---
# Story: codespec-spans/1, crates/edit and `codespec read --spans`

## Outcome

A new crate `crates/edit` (`b10x-codespec-edit`, a library; dependencies `b10x-codespec-model`,
`b10x-codespec-query` and the workspace's `sha2`) starts with three modules and declares them in
`src/lib.rs`:

- `spans`: the `codespec-spans/1` document of `docs/code-edit-language.md` § 6.2 and its strict
  reading against its `cir/1` document and the source files (format, no unknown field,
  `cir_sha256`, each file's SHA-256 and length, spans inside their file on UTF-8 boundaries, `line`
  and `end_line` consistent with the bytes, `text` equal to the bytes, every `at` naming a node of a
  kind its role allows, no overlapping `name` spans, and completeness: a `decl` span for every
  function and a `name` span for every node of a kind wave 1 renames, or an `unspanned` entry).
  Canonical encoding, sorted by `at` then `role`.
- `ids`: the two-way mapping between `at` paths and the ids cq gives, for every kind an edit
  names: a function id `<module path>.<key>` and `funcs.<i>` or `opaque.<i>`; a type id and
  `types.<i>`; a struct field (type id and field name) and `types.<i>.fields.<j>`; a node id
  `<module path>.<key>#<index path>` and `funcs.<i>.body.<index path>`. From these, `source_names`
  gives the `source_name(id, text)` records for the fact base (`story:edit-cq-items`): the `text` of
  each `name` span under the cq id of its node, function or field.
- `run`: the one module of the crate that starts processes and touches files (the other modules
  stay pure). Here it holds `read_spanned(reader, dir)`: run the reader with `--spans`, take its
  object `{cir, files, spans, unspanned, refs}` (§ 6.3), strict-read the `cir/1`, add `module` and
  `cir_sha256`, strict-read the span document. A reader that fails on `--spans` or prints a bare
  module is refused: "the <lang> reader gives no spans". `story:edit-slice-rust-rename` extends
  `run` to the whole pipeline, so `crates/cli` and `crates/mcp` both call it.

`codespec read --spans <out> --lang <l> <dir>` is a thin call of `run::read_spanned` that writes both
documents. The adapter protocol is written in `docs/design.md` § 2.

## Done When

- `crates/edit/tests/main.rs` (the crate's one test target): one refusing test per strict-reading
  rule above, each naming the rule and the `at` in its message; a valid document reads and
  re-encodes byte-identically.
- Same target, over `examples/series/series.read.cir.json` and its fact base: every function,
  type, struct field and body node maps from its cq id to its `at` and back to the same id;
  `source_names` over a hand-written span document gives the expected records, one per `name`
  span, keyed by function id, field and node id.
- Same target: `docs/design.md` § 2 holds the strings `--spans`, `cir`, `files`, `spans`,
  `unspanned` and `refs` (a test reads the file and cuts the section at its heading).
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

- Round 1: `--overlay` removed (re-reading uses a cache copy); no `AdapterManifest` change, a reader
  without spans is detected by its output; `sha2` lives in `crates/edit`; `crates.md` in scope; tests
  in an existing test binary; the mapping is owned here; `node` spans dropped from wave 1.
- Round 2, `docs/design.md` § 2 untested: the string test above.
- Round 2, mappings only below `funcs.<i>.body`: `ids` covers functions, opaque functions, types
  and fields, and `source_names` gives the fact-base records.
- Round 2, orchestration in a library: the `run` module, extended by the slice.

## Scope

- Own: `crates/edit/` (new: `Cargo.toml`, `src/lib.rs`, `src/spans.rs`, `src/ids.rs`, `src/run.rs`,
  `tests/main.rs`, `tests/fixtures/`), the workspace `Cargo.toml` (members), `Cargo.lock`,
  `scripts/check-parts.txt`, `crates/cli/Cargo.toml`, `crates/cli/src/lib.rs` and
  `crates/cli/src/main.rs` (`Read` flags), `crates/cli/tests/query.rs` (the `mod edit;` line),
  `crates/cli/tests/edit/` (new), `crates/cli/tests/fixtures/edit/`, `docs/design.md` § 2,
  `website/docs/reference/cli.md` and `crates.md` (generated), `CHANGELOG.md`.
- First story of the wave. It writes `Cargo.lock` with every dependency `crates/edit` needs in wave
  1, so later stories of the crate add none.
