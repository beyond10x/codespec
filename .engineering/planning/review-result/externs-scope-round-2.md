---
format: aep.planning-md/3
id: review-result:externs-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
approve

**What you read:** the epic and 4 stories (all bodies), `docs/static-analysis.md` §§ 3–5, and the repo `AGENTS.md` lines on docs and capabilities. I ran `aep plan artifact show` on each artifact, `aep plan artifact graph` and `aep plan artifact findings epic:externs-and-effects`. The graph showed no other item claiming any part of the epic. The findings command found no earlier review of the epic to compare against.

**Promise count:** I extracted 5 promises from the epic and traced all 5 to an item. Each promise below is quoted from the epic body.
1. "A Go package that calls the standard library is read with its library calls as typed `extern` calls instead of `foreign` statements" is claimed by `story:go-extern-calls`, on top of the model addition in `story:cir1-externs`.
2. "`codespec effects` reports per function which effects … it may have, from checked-in summaries of the Go standard library" is claimed by `story:effects-report`.
3. "every call or construct it cannot account for listed as `unknown` at its position" is claimed by `story:effects-report`. Its `unknown` reasons are extern without summary, `foreign` statement, method on an undeclared type, and a callee that is incomplete.
4. The Done When item that `reader-go.json` and `writer-go.json` list `expr.extern` as claimed and `pass`, and that the Rust writer's report lists it as not claimed and `refused`, is split. `story:go-extern-calls` claims the Go reports and the Rust writer's report. `story:cir1-externs` claims the Rust writer's refusal and the `not` lists.
5. The Done When item that `task examples-effects` reads `examples/envcheck` and its report equals `envcheck.effects.json`, with `env`, pure and `unknown` functions, is claimed by `story:effects-example`.

**Out of Scope check:**
- No story reaches into the Rust, Java or Groovy readers beyond their `not` manifest entries. The epic requires those entries.
- No story reaches into dependency-source summaries or a traced-run check.
- `story:go-extern-calls` keeps variadic functions and functions with parameters of other packages' types as `foreign`, which matches the exclusions.
- The README, site index, `CHANGELOG.md` and capability entry in `story:effects-example` are not traceable to a sentence in the epic. `AGENTS.md` lines 106–114 require them for any new example or CLI change, so I did not treat them as reach.

**What you could not establish:**
- Both `story:cir1-externs` and `story:go-extern-calls` assert something about `writer-rust.json` (`not` claimed in one, `not` claimed and `refused` in the other). The second reads as a regeneration check rather than a second claim, so I did not make it a finding.
- Out of my lane, and none of it sets the verdict: whether each acceptance is checkable, whether `story:effects-example` has a hidden coupling on the shared `summaries/go-std.effects.json`, and whether `task check` stays green between commits.

```findings
[]
```
