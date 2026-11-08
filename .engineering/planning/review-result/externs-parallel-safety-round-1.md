---
format: aep.planning-md/3
id: review-result:externs-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
approve

**What you read:** 5 artifacts (the epic and 4 stories). Commands: `aep plan artifact show` for each, `aep plan artifact graph`, `aep plan artifact waves`, plus `git grep` and reads of `Taskfile.yml` and `crates/docs/src/main.rs`. Each story's `## Scope` section cites its surfaces, so all 4 stories are placed from their bodies.

- **Surfaces:** 4 cited, 0 inferred, 0 unplaceable.
- **Edges:** the `depends_on` edges are `go-extern-calls` and `effects-report` onto `cir1-externs`, and `effects-example` onto both `go-extern-calls` and `effects-report`.
- **Collision check:** `cir1-externs` shares files with `go-extern-calls` and with `effects-report`. Every such pair is ordered by an edge. The one pair with no edge, `go-extern-calls` and `effects-report`, lands on disjoint files:
  - `go-extern-calls`: `adapters/go/cmd/…`, `conformance/…`, and the generated `website/data/conformance/*` and support matrix.
  - `effects-report`: `crates/effects`, `crates/cli`, `Cargo.lock`, `summaries/`, and the generated `cli.md` and `crates.md`.
- **Reads traced to a producer:** 5 acceptance reads traced to a producer in the set, and a `depends_on` edge records all 5. They are:
  - `go-extern-calls` reads `writer-rust.json` from `cir1-externs`.
  - `effects-report` reads `Expr::Extern` from `cir1-externs`.
  - `effects-example` reads the Go reader's extern output from `go-extern-calls`.
  - `effects-example` reads the `codespec effects` command from `effects-report`.
  - `effects-example` reads `summaries/go-std.effects.json` from `effects-report`.

**What you could not establish:**
- None that changes the verdict.
- The four stories have no machine `scope` field. `aep plan artifact waves` prints all four under `unassessed` and places none of them. This is the later scoping step (SKILL § 8), not a defect in the bodies.
- Out of my lane, for the scope and acceptance critics:
  - `cir1-externs` adds `expr.extern` to the vocabulary, which makes `conformance/kit.json` stale (`Taskfile.yml:339` compares it to the generated kit). Only `go-extern-calls` lists that file, and `cir1-externs` acceptance runs neither `conformance-cases` nor `conformance-kit`.
  - `effects-report` says `summaries/go-std.effects.json` covers the `strings.*` functions "used by examples and kit cases". `examples/envcheck` and `conformance/cases/expr-extern` do not exist until later stories, and `effects-example` lists no edit to the summaries file.

```findings
[]
```
