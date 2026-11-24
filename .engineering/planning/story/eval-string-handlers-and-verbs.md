---
format: aep.planning-md/3
id: story:eval-string-handlers-and-verbs
kind: story
status: draft
title: The evaluator runs the string externs and format verbs a service calls
summary: Built-in handlers for strings.Contains, HasPrefix, HasSuffix, Split, ReplaceAll, EqualFold and strconv.Quote, and sprintf with %x, %X, %q, %t, widths and flags.
relations:
- decomposes: epic:cir-evaluator
- informed_by: story:eval-handlers-traces
revision: 2
---
# Story: The evaluator runs the string externs and format verbs a service calls

## Outcome

`codespec eval` has built-in handlers, each held to a kit case, for `strings.Contains`,
`strings.HasPrefix`, `strings.HasSuffix`, `strings.Split`, `strings.ReplaceAll`,
`strings.EqualFold` and `strconv.Quote`, with Go's results; and `sprintf` takes `%x`, `%X`,
`%q`, `%t` and widths and flags (`%6s`, `%-4s`, `%02d`) as Go prints them. A verb or extern
outside the table still stops the run naming it.

## Why

A consumer that writes `cir/1` documents by hand from a specification (97 documents,
2026-11-24) lists among its tool limits: "`eval` … formats `%d %s %v` only, has handlers for
`strings.TrimSpace`/`ToLower` but not `Split`, `Contains`, `HasPrefix`, `ReplaceAll`,
`EqualFold`, `strconv.Quote` (such documents validate but are not run)". The Rust reader already
writes `%02d`, `%-4s` and `%6s` for Rust's `{:02}`, `{:<4}` and `{:>6}`, so documents it reads
stop there too. Reproduced on main `fff33d171` with a document written from scratch:

- `strings.Contains("abc", "b")`: "no handler: extern strings.Contains has no handler" (the
  same for `HasPrefix`, `EqualFold`, `strconv.Quote`);
- `sprintf("%x", 255)`: "unsupported: sprintf verb %x is outside the evaluator's %d, %s, %v";
- `sprintf("[%6s]", "ab")`: "unsupported: sprintf verb %6 is outside the evaluator's %d, %s,
  %v".

## Done When

- A failing test first in `crates/eval/tests/`: the calls above give Go's results (`true`,
  `"\"a\""`, `ff`, `[    ab]`), red on main.
- The handler table in `crates/eval/src/builtins.rs` and `docs/evaluator.md` § 6.1 list each
  new handler and verb with its kit case.

## Scope

- `crates/eval/src/builtins.rs`, `crates/eval/src/`, `crates/eval/tests/`, `conformance/`,
  `docs/evaluator.md`, `CHANGELOG.md`. Inferred. Reproducer: `repro/tools/` (not committed).
