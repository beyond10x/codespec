---
format: aep.planning-md/3
id: review-result:externs-design-round-2
kind: review-result
status: active
title: Design critic, round 2
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
approve

What I read: the epic and the four stories (5 artifacts), plus `aep plan artifact relations`, `graph` and `validate`. `validate` reports `valid`. I also read `review-result:externs-design-round-1` and the verdict table in `crates/conformance/src/conform.rs` at HEAD.

Both round-1 findings are closed:
- **Split feature.** `story:cir1-externs` now puts `expr.extern` in `not` for the Go reader and writer until `story:go-extern-calls`, which removes it and adds the case and `kit.json` entry. An unclaimed feature with no case is `untested`, not `fail`, so each story lands with `task check` green.
- **Summary file.** `story:effects-report` now lists the `strings` and `strconv` functions by name. `story:effects-example` also has `summaries/go-std.effects.json` in its scope, for any function it needs that `story:effects-report` did not list.

Shape of the set:
- **Cycle and chain.** There is none. I walked 8 declared edges in the set, 4 `decomposes` and 4 `depends_on`, and read the whole graph, so I did walk outside the set. The shape is a diamond. `story:cir1-externs` comes first, then `story:go-extern-calls` and `story:effects-report` can run in parallel, then `story:effects-example`.
- **Acceptance reads.** I traced 7 reads of state another item in the set produces. A `depends_on` edge records all 7:
  - `story:go-extern-calls` reads the Rust writer's `refused` verdict from `story:cir1-externs`.
  - `story:effects-example` reads the Go reader's output from `story:go-extern-calls`.
  - `story:effects-example` reads the summary file and the `codespec effects` command from `story:effects-report`.
  - The other 3 are the `story:go-extern-calls` and `story:effects-report` reads of the model types, the `Expr::Extern` check and the manifests from `story:cir1-externs`.
- **Split abstraction.** None. `story:cir1-externs` holds the model and the refusals. `story:go-extern-calls` holds the Go reader and writer that use them. `story:effects-report` holds the analysis, which needs only a `Module`.

What I could not establish:
- `story:effects-example` and `story:effects-report` both edit `summaries/go-std.effects.json`. The `depends_on` edge between them covers that order, and I count it as a stated trade-off, not a chain. Whether the two can run at once is the parallel-safety critic's question.
- Out of my lane, for the acceptance critic: the `story:cir1-externs` acceptance says all six manifests list `expr.extern` under `not`. That stops holding once `story:go-extern-calls` lands. I did not let it set my verdict.

```findings
[]
```
