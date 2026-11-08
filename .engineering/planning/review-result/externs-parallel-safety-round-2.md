---
format: aep.planning-md/3
id: review-result:externs-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
approve

I read the revised set and found no unrecorded collision, no missing producer edge and no item left without a surface.

**What you read:** 5 artifacts (the epic and the 4 stories). Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact waves`, and the round-1 `review-result:externs-parallel-safety-round-1`. I also read `Taskfile.yml` (`check`, `docs-generate`, `docs-check`, `examples-check`) and ran `git grep` over `crates/docs/src` and the generated site pages.

- **Surfaces:** 4 cited, 0 inferred, 0 unplaced. Each story's `## Scope` names its paths.
- **Edges:** `go-extern-calls` and `effects-report` each depend on `cir1-externs`. `effects-example` depends on both `go-extern-calls` and `effects-report`.
- **Shared files, all ordered by an edge:**
  - `cir1-externs` and `go-extern-calls` share the two Go `describe.go` files, `conformance/kit.json` and the conformance reports.
  - `effects-report` and `effects-example` share `summaries/go-std.effects.json`.
  - `effects-example` shares the generated site files with the earlier stories.
- **The one unordered pair:** `go-extern-calls` and `effects-report`. Their surfaces are disjoint:
  - `go-extern-calls` touches `adapters/go/…`, `conformance/…`, `website/data/conformance/*`, `website/data/support-matrix.json` and `support.mdx`.
  - `effects-report` touches `crates/effects`, `crates/cli`, `Cargo.toml`, `Cargo.lock`, `summaries/`, `reference/cli.md` and `reference/crates.md`.
  - Both say "Generated site files", but `crates/docs/src` generates those two groups of pages independently. `status.json` changes only in `effects-example`, through its new capability.
- **Reads traced to a producer in the set:** 5, and the `depends_on` edges record all 5.
  - `go-extern-calls` reads `writer-rust.json` from `cir1-externs`.
  - `effects-report` reads `Expr::Extern` from `cir1-externs`.
  - `effects-example` reads the Go reader's extern output from `go-extern-calls`.
  - `effects-example` reads the `codespec effects` command from `effects-report`.
  - `effects-example` reads `summaries/go-std.effects.json` from `effects-report`.
- **Round-1 gaps now closed:**
  - `go-extern-calls` and `effects-example` now list `conformance/kit.json` and `summaries/go-std.effects.json` as scope.
  - `cir1-externs` now says the Go reader and writer stay in `not` until `go-extern-calls`.

**What you could not establish:**
- The stories still have no machine `scope` field, so `aep plan artifact waves` lists all four as `unassessed`. That is the later scoping step, not a defect in the bodies.
- Out of my lane, and it does not set the verdict: `story:go-extern-calls` says in Context that it removes `expr.extern` from the Go `not` lists. Its `## Scope` does not list `adapters/go/cmd/codespec-read-go/describe.go` or `adapters/go/cmd/codespec-write-go/describe.go`, which `story:cir1-externs` lists. An edge orders the pair, so this is not a collision. The scope critic may want the two paths in the list (`.engineering/planning/story/go-extern-calls.md`, Context and Scope).

```findings
[]
```
