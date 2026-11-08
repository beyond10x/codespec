---
format: aep.planning-md/3
id: story:link-modules
kind: story
status: implemented
title: codespec effects analyses a set of modules linked by import path
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:cir1-extern-types
- depends_on: story:foreign-calls
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/encode.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/ir.go
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/tests/effects.rs
- confidence: cited
  path: crates/effects/
- confidence: cited
  path: crates/effects/tests/effects.rs
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: schemas/cir-1.schema.json
- confidence: cited
  path: scripts/read-repo.sh
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:56Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:57Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:29Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: codespec effects analyses a set of modules linked by import path

## Outcome

`codespec effects a.cir.json b.cir.json … [--summaries …]` reads several documents as one program:
an extern call naming `(path, name)` where `path` is the import path of a document in the set is
a call of that document's function, and effects are computed to one fixed point over the whole set.

## Context

- `docs/static-analysis.md` § 5, "Several modules at once". One module per document stays.
- `Module.path` (import path), optional and additive; the Go reader writes it.
- `crates/effects`: `analyze_set(&[Module], &[Summary]) -> Report`; a report entry is keyed by
  `path` plus `Func::key`. A summary entry and a document both defining one extern: the document
  wins, and the report notes it.
- `scripts/read-repo.sh` gains `--deps`: also reads every package of `go list -deps`.

## Acceptance

- `crates/effects/tests/effects.rs`: two modules, `a` calling `b.F` which calls `os.Getenv`;
  `a`'s caller reports `env` with no `unknown`; with `b` missing it reports
  "extern without summary: b.F".
- `scripts/read-repo.sh --deps` on the measured library exits 0 for all 234 packages (6 of the library,
  228 imported, `go list -deps`).
- `crates/cli/tests/effects.rs`: the command accepts several documents; output is byte-identical
  across two runs.
- `task check` passes.
