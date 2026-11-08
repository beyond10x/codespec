---
format: aep.planning-md/3
id: review-result:externs-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
needs-revision

story:effects-report — the acceptance checks nothing about the checked-in Go standard library summary, so the Outcome's "A Go standard library summary is checked in" can be called done with `summaries/go-std.effects.json` absent, unparsed, or missing the functions named in Context — `.engineering/planning/story/effects-report.md:19`
story:effects-report — the acceptance checks `codespec effects` only through `--help` and a hand-built module in `crates/effects`, so nothing observable shows that the command run on a `module.cir.json` with `--summaries` prints `codespec-effects-report/1` — `.engineering/planning/story/effects-report.md:45`

What I read: 5 artifacts (the epic and the 4 stories), with `aep plan artifact show <id>` for each. I also ran `aep plan artifact kinds` and `aep plan artifact lifecycle story`, and used `git grep` on `Taskfile.yml`, `adapters`, `website/data/conformance` and `crates/cli/src`.

What I could not establish:
- Each story's acceptance is a bullet list of independent checks, not one sentence. I did not flag it because the other stories in this store follow the same pattern and every bullet can be checked by a command in `task check`.
- Out of my lane, for the design critic: `story:cir1-externs` adds `expr.extern` to the vocabulary, and the reports in `website/data/conformance` list one entry per feature. The kit case `expr-extern` and the `reader-go`/`writer-go` claims only arrive in `story:go-extern-calls`. Its `task docs-check` bullet may not hold until that story lands. I did not check this.
- The unknown reasons `method on undeclared type` and `calls <func> which is incomplete` have no test in the acceptance. I treated that as thin coverage, not a finding.

```findings
- file: .engineering/planning/story/effects-report.md
  line: 19
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks nothing about the checked-in Go standard library summary, so the Outcome's 'A Go standard library summary is checked in' can be called done with summaries/go-std.effects.json absent, unparsed, or missing the functions named in Context"
- file: .engineering/planning/story/effects-report.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks codespec effects only through --help and a hand-built module in crates/effects, so nothing observable shows that the command run on a module.cir.json with --summaries prints codespec-effects-report/1"
```
