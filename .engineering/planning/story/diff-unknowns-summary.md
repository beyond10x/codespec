---
format: aep.planning-md/3
id: story:diff-unknowns-summary
kind: story
status: draft
title: codespec diff --unknowns summary groups unknown deltas by reason
summary: A mode that replaces unknown-added and unknown-removed deltas by one count per reason class
relations:
- decomposes: epic:release-diff-go-field-gaps
- depends_on: story:diff-command
scope:
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/diff
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 2
---
# Story: codespec diff --unknowns summary groups unknown deltas by reason

## Outcome

`codespec diff --unknowns <mode>` (`list`, the default, or `summary`). With `summary`, the deltas of
kinds `unknown-added` and `unknown-removed` are not printed one per line; the report holds, in
their place, one entry per (kind, reason class) with its count, and the other deltas are
unchanged.

The reason class is the reason text of the delta's `value` up to its first `: `; a reason with no
`: ` is its own class. A table of the families (the strings of `unknown(..)` in
`crates/effects/src/set.rs`: `method on undeclared type`, `extern without summary`, `not in its
module`, `call of undeclared function`, `interface call with no implementation in the set`,
`function value with no candidate in the set`, `foreign statement`, and the rest) lives beside
those strings in `crates/effects` as one function, `Unknown::class`, so a new reason is added with
its class and a test lists every reason the analysis can make.

`codespec-diff-report/1` gains, only with `--unknowns summary`, the field `unknowns`: an array
sorted by `kind`, then `class`, of `{kind, class, count, units, examples}`; `count` is the deltas
of that class, `units` the number of distinct functions they are of, `examples` the first three
subjects (the part after the first `: `) in the report's delta order. The text form prints
`unknown-added <class> count=<count> units=<units>`, one line each, then up to three
indented examples. The `deltas` array holds no `unknown-*` delta in this mode. Exit code: 1 when
`deltas` or `unknowns` is not empty, as with the default mode; 0 with none.

A mode that drops lines needs a way back: `--unknowns list` is today's output and the default.
Decision D1, default A: a mode of its own on `codespec diff`. B (not taken): a separate command
`codespec diff-summary`, which duplicates every option of `diff`.

## Why

234 of 293 deltas of a release review of a Go service were of kind `unknown`; each is one line, and
a reviewer wants counts per reason, then the units. The grouping is also the unit of work for
`story:diff-unknown-reasons-measured`, which turns each frequent class into a story.

## Reproducer

Two hand-built revisions of `example.com/probe` as `cir/1` documents (as the tests of
`crates/effects` build them): `b` adds three functions, each calling a different extern no summary
names (`example.org/lib.A`, `B`, `C`: each an `extern without summary`), and one function holding
a `foreign` statement with no call list. `codespec diff a-docs b-docs --unknowns summary`.

## Observed

Not run: `codespec diff` does not exist on main until `story:diff-command` is implemented. By its
design, `unknown-added` carries one delta per reason, so `list` prints four of them here.

## Done When

- A failing test first in the integration-test binary of `crates/diff` (`tests/main.rs`): the
  reproducer with `--unknowns summary` gives `unknown-added extern without summary count=3
  units=3` and `unknown-added foreign statement count=1 units=1`; with `list` (and without the
  flag) it gives today's four lines.
- Near misses: a reason without `: ` is its own class; two reasons of one class on one function
  count twice and the function once in `units`; an `unknown-removed` of the same class is its own
  entry; no unknown deltas gives no `unknowns` field and exit 0 when nothing else changed; the same
  inputs twice give the same bytes; `--format json` and text hold the same counts.
- A test in `crates/effects` gives, for each shape of call site that makes an `unknown` reason
  (one fixture per `unknown(..)` call of `set.rs`), a class; two reasons that differ only in the
  symbol they name give one class (a class is a family, not a symbol).
- `codespec diff --help`, `website/docs/reference/cli.md` and the crate doc of `b10x-codespec-diff`
  (the report format) state the mode; `task docs-generate` run; `task check` green; each new test
  under 60 s and the test step within the 120 s budget.

## Scope

- Own: `crates/diff/` (the summary, the report field, `tests/main.rs`), `crates/effects/src/lib.rs`
  (`Unknown::class`) and its tests, `crates/cli/src/lib.rs` and `main.rs` (the flag),
  `website/docs/reference/cli.md`, `website/docs/reference/crates.md`. Inferred; recorded as typed
  scope. `CHANGELOG.md` gets one line, outside the typed scope.
- After `story:diff-command`, which creates `crates/diff` and the report: a `depends_on` edge.
- Shares `crates/effects/src/lib.rs` with `story:reach-inits-per-program`; whichever starts second
  rebases on the other.
