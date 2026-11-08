---
format: aep.planning-md/3
id: epic:externs-and-effects
kind: epic
status: implemented
title: 'Externs and effects: cir/1 names the calls it leaves, and an analysis reports what they do'
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T01:38:27Z", actor: "human:timo", revision: 5}
---
# Epic: Externs and effects: cir/1 names the calls it leaves, and an analysis reports what they do

## Outcome

A Go package that calls the standard library is read with its library calls as typed `extern`
calls instead of `foreign` statements, and `codespec effects` reports per function which effects
(`fs.read`, `env`, `stdout`, …) it may have, from checked-in summaries of the Go standard library,
with every call or construct it cannot account for listed as `unknown` at its position. Design:
`docs/static-analysis.md` § 3 and § 4.

## Why Now

`cir/1` is typed and read from four languages, and the passes in `crates/passes` are already
analyses, but `Expr::Call` can only name a function of the module (`crates/model/src/lib.rs`, the
doc of `Expr::Call`). Every call into a library, the point where a program does IO, is kept
`foreign` (`adapters/go/cmd/codespec-read-go/front.go`, `call`), so no analysis can say what a
program does beyond its own arithmetic.

## Out of Scope

- Summaries computed from a dependency's source, and analysis across several modules
  (`docs/static-analysis.md` § 5).
- Checking summaries against a traced run (`docs/static-analysis.md` § 6).
- Extern calls in the Rust, Java and Groovy readers: they list `expr.extern` in `not`.
- Writing an extern call as Rust: the Rust writer refuses it.
- Variadic externs, methods on types of other packages, function values.

## Done When

- `website/data/conformance/reader-go.json` and `writer-go.json` list `expr.extern` as claimed and
  `pass`; the Rust writer's report lists it as not claimed and `refused`.
- `task examples-effects` reads `examples/envcheck` (a Go package calling `os`, `strings` and
  `strconv`), and its `codespec effects` report equals the checked-in `envcheck.effects.json`,
  with at least one function reported with `env`, one pure, and one `unknown`.
