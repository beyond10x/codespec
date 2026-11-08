---
format: aep.planning-md/3
id: review-result:externs-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
approve

I extracted 5 promises from the epic and traced all 5 to an item. Nothing in the set reaches beyond the epic's text or its Out of Scope list, and no outcome is claimed twice.

1. Outcome, Go stdlib calls read as typed `extern` instead of `foreign` (epic lines 13-14): `story:go-extern-calls`, on top of `story:cir1-externs`, which supplies the model, strict checks and manifests.
2. Outcome, `codespec effects` reports per function from checked-in Go stdlib summaries (lines 14-16): `story:effects-report`.
3. Outcome, everything unaccounted for listed as `unknown` at its position (line 16): `story:effects-report`, which defines the `{pos, reason}` entries.
4. Done When, `reader-go.json` and `writer-go.json` claimed and `pass`, with the Rust writer not claimed and `pass` (refused) (lines 38-39): `story:go-extern-calls` acceptance for the three conformance reports, and `story:cir1-externs` acceptance for the Rust writer refusal.
5. Done When, `task examples-effects` on `examples/envcheck`, with one `env`, one pure and one `unknown` function (lines 40-42): `story:effects-example`.

The out-of-scope items agree with the stories:
- The Rust, Java and Groovy `not` manifests sit in `story:cir1-externs`.
- The refusal of variadic calls and of other-package parameter types sits in `story:go-extern-calls`.
- Sections 5 and 6 of the design are claimed by nothing.

The pieces that look like reach all trace back to the epic or to a repository rule:
- The walk in `crates/passes` follows from adding the new `Expr::Extern` variant.
- The `status.rs` capability, `README.md`, index and `CHANGELOG.md` entries follow the generated-docs and shipped-capability rules in `AGENTS.md` (lines 106-114, 159).

What I read: 5 artifacts (the epic and four stories) and `docs/static-analysis.md` (§ 3-§ 6). I ran `aep plan artifact show` on each, `aep plan artifact graph`, `git status`, and `ls`/`grep` over `adapters`, `Taskfile.yml` and `AGENTS.md`. The graph shows no other artifact that claims part of this epic.

What I could not establish:
- The epic says "summaries of the Go standard library", but `summaries/go-std.effects.json` is only "at least" a named list. I treated this as covered by the epic's `unknown` rule for unsummarised externs and did not count it as a silent narrowing.
- Whether the acceptances are checkable, whether the set holds together, and whether two stories can run in parallel belong to the other critics. I did not judge them.

```findings
[]
```
