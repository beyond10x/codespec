---
format: aep.planning-md/3
id: review-result:externs-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
needs-revision

story:effects-example — the Outcome says `task examples-effects` "holds the report in `task check`", but no acceptance bullet shows `task check` runs it, so the story can close with `examples-effects` passing on its own and absent from the `check` task — .engineering/planning/story/effects-example.md:19
story:effects-example — "`task docs-check` passes with the new capability" reads the same before the work (no capability, passes) as after it, so it never shows the capability naming `Taskfile.yml::examples-effects` exists — .engineering/planning/story/effects-example.md:37

What I read: 5 artifacts (the epic and the 4 stories), each in full with `aep plan artifact show <id>`. I also read round 1's `review-result:externs-acceptance-round-1`. I used `git grep` and file reads on `Taskfile.yml`, `website/data/conformance/*.json` and `conformance/cases/stmt-foreign`.

Round 1 findings, both on `story:effects-report`: both landed.
- Line 19 (the summary file): a bullet now reads `summaries/go-std.effects.json` strictly and asserts every function in Context.
- Line 45 (the command): `crates/cli/tests/effects.rs` now runs `codespec effects <file> --summaries …` and checks the output document, exit 0, byte-equality on a second run, and a non-zero exit for a bad summary file.

What I could not establish:
- Each story's acceptance is still a bullet list of independent checks, not one sentence. I did not flag it, for the reason I gave in round 1.
- `story:effects-report` tests only one `unknown` reason, `extern without summary`, although its Outcome says "every reason that set may be incomplete". `foreign statement` is covered in `story:effects-example`. `method on undeclared type` and `calls <func> which is incomplete` have no test anywhere. I treated this as thin coverage, as in round 1.
- Whether `writer-rust.json` can report `refused` for `expr.extern`: the existing `stmt.foreign` entry shows the report produces a `refused` verdict from a case's `case.cir.json`, so I took the claim as plausible.
- Out of my lane: `story:cir1-externs` regenerates the kit and reports with `expr.extern` before the `expr-extern` case exists (it arrives in `story:go-extern-calls`). Whether `task check` and `docs-check` hold in between is for the design critic.

```findings
- file: .engineering/planning/story/effects-example.md
  line: 19
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says `task examples-effects` holds the report in `task check`, but no acceptance bullet shows `task check` runs it, so the story can close with `examples-effects` passing on its own and absent from the `check` task"
- file: .engineering/planning/story/effects-example.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`task docs-check` passes with the new capability reads the same before the work (no capability, passes) as after it, so it never shows the capability naming Taskfile.yml::examples-effects exists"
```
