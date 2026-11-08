---
format: aep.planning-md/3
id: story:effects-report
kind: story
status: implemented
title: Effect summaries and the codespec effects report
relations:
- decomposes: epic:externs-and-effects
- depends_on: story:cir1-externs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-08T01:38:27Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
# Story: Effect summaries and the codespec effects report

## Outcome

`codespec effects <module.cir.json> --summaries <file>…` prints `codespec-effects-report/1`: per
function, the effects it may have and every reason that set may be incomplete, from
`codespec-effects/1` summary documents (`docs/static-analysis.md` § 4). A Go standard library
summary is checked in.

## Context

- The summary format and the report live in a new crate `crates/effects`
  (`b10x-codespec-effects`): strict reading of `codespec-effects/1`, the closed effect vocabulary,
  the analysis as a function over a `Module` and a set of summaries, canonical JSON output.
- The analysis: each function's effects are those of the externs it calls, of `println` (`stdout`)
  and of every module function or method it calls, to a fixed point. A method call resolves to
  the module method of the receiver's named type (through a pointer); a receiver type the module
  does not declare is `unknown`.
- `unknown` entries: `{pos, reason}` with reason `extern without summary: <path>.<name>`,
  `foreign statement`, `method on undeclared type: <type>.<name>`, or `calls <func> which is
  incomplete`, sorted by position then reason.
- `summaries/go-std.effects.json`: at least `os.Getenv`, `os.LookupEnv`, `os.ReadFile`,
  `os.Exit`, `os.Getwd`, `os.Hostname`, `strings.Repeat`, `strings.ToUpper`, `strings.ToLower`,
  `strings.TrimSpace`, `strings.Contains`, `strings.HasPrefix`, `strings.Split`, `strings.Join`,
  `strconv.Itoa`, `strconv.Atoi`, `time.Sleep`, `math/rand.Intn`, `net.LookupHost`.

## Acceptance

- `crates/effects` tests: a hand-built module where `a` calls `b`, `b` calls extern `os.Getenv`,
  `c` calls an extern without summary and `d` calls only `strings.ToUpper` gives `a` and `b`
  `["env"]`, `c` an `unknown` naming the extern, `d` `[]` with no `unknown`; a recursive pair
  terminates with the union; the report run twice gives the same bytes.
- A summary with an unknown effect, an unknown field, a duplicate extern or another format is
  refused, naming it.
- `summaries/go-std.effects.json` reads strictly as `codespec-effects/1` and holds every function
  Context lists: a `crates/effects` test reads the checked-in file and asserts each one.
- `crates/cli/tests/effects.rs`: `codespec effects <file> --summaries summaries/go-std.effects.json`
  on a module file prints a `codespec-effects-report/1` document, exit 0, the same bytes on a
  second run; a missing or refused summary file exits non-zero naming it.
- `codespec effects --help` names `<MODULE>` and `--summaries`; `website/docs/reference/cli.md`
  regenerated; `task docs-check` passes.

## Scope

- `crates/effects/` (new), `Cargo.toml` (workspace member), `Cargo.lock`.
- `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/Cargo.toml`, `crates/cli/tests/effects.rs` (new).
- `summaries/go-std.effects.json` (new).
- Generated site files (`task docs-generate`).
