---
format: aep.planning-md/3
id: story:reader-c-to-error-variadic
kind: story
status: implemented
title: The C reader claims expr.to_error and decl.variadic
relations:
- decomposes: epic:reader-coverage
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T20:14:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T20:14:35Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-10T22:06:36Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
# Story: the C reader claims expr.to_error and decl.variadic

## Outcome

`codespec-read-c` reads the runtime form `cs_error_of` as `expr.to_error` and a `va_list`
function in the runtime form the C writer writes as `decl.variadic`, each through a kit case
source under `conformance/cases/<case>/c/`, so the C `read` column goes from 103 to 105 verified.

## Why

Both have a C source form in the convention the C writer writes, and the crate doc of
`adapters/c-read` lists them as not read. They are two of the five features between the C reader
and every feature C can write.

## Acceptance

- `codespec conform --lang c --role reader` reports `expr.to_error` and `decl.variadic` as
  `pass`, 105 claimed, 105 pass, 0 fail, `conforms: true`; every feature claimed before is still
  `pass`.
- The reader refuses a near miss of each form (a `cs_error_of` of a non-error value; a `va_list`
  read with a type other than the declared element type) with a refusal at its line; a test per
  near miss.
- If one of the two cannot be read without guessing, it stays unclaimed with its reason in the
  crate doc, and the story reports 104.
- `cargo test -p codespec-read-c`, `task conformance`, `task adapters-check`, `task c-round-trip`
  exit 0.

## Scope

- Own: `adapters/c-read`, `conformance/cases/*/c/` (only new files of language C), `CHANGELOG.md`.
  `adapters/c-run` only where a new case source needs the runner to call it. Inferred.
