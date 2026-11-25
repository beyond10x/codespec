---
format: aep.planning-md/3
id: review-result:srules-scope-round-2
kind: review-result
status: active
title: Plan critic scope, round 2, epic security-rules-in-codespec
relations:
- reviews: epic:security-rules-in-codespec
- reviews: story:rules-layout
- reviews: story:rules-sanitization-gate
- reviews: story:rules-third-party-attribution
- reviews: story:rules-runner
- reviews: story:rules-import-mechanics
- reviews: story:rules-sanitize-and-fixtures
- reviews: story:rules-climb-lane
- reviews: story:rules-catalogue-docs
- reviews: story:rules-planning-merge
- reviews: story:rules-ui-crate
- reviews: story:codesec-retirement
revision: 1
---
needs-revision
`.engineering/planning/story/rules-climb-lane.md` — the title ("…and the reader corpus loop runs the rules") and summary ("rule KPIs in the reader corpus loop") promise the owner's ask that the public reader corpus loop runs the rules, but the body hands over only the `summary.json` interface and defers the loop's rule step to an epic not in this store, and neither the epic's Out of scope nor the story's Done When records that part as not covered (retitle, or record it as not covered in the epic and the story) — `.engineering/planning/story/rules-climb-lane.md:87`
`epic:security-rules-in-codespec` — the inventory table says "2 scripts" move as is, but no story claims moving them: `story:rules-runner` says the runner replaces codesec's hunt task and a shell script with a Rust command, `story:rules-import-mechanics` adds new scripts, and no scope lists a moved script (name the story that takes them, or change the row to "rewrite" or "drop") — `.engineering/planning/epic/security-rules-in-codespec.md:47`

Round 1 findings:
- Finding 1, sanitizing the 13 measurement data and prose files: fixed. `story:rules-climb-lane` now checks each file against the inventory's list, runs it through the structural check of `story:rules-sanitization-gate` (which now names `rules/climb/` and an allow list of public repositories), and has a Done When for `codespec rules check` over `rules/climb/`. The epic's table points the 13 files at it.
- Finding 2, the E7 cell: closed as a deliberate owner decision. The epic's "Open, not taken by this epic" paragraph and `story:rules-catalogue-docs` both record that the comparison is unpublished here. The `active` count in `website/data/rules.json` is the one source, and the story has a Done When for the count. I do not count it against the set.

Other points checked and not findings:
- Issues 13 and 14 are mapped in `story:rules-planning-merge`. Every codespec story it names exists in the tree, and the two new stories are in its scope.
- The planning-merge counts add up: 8 + 15 + 27 + 21 + 4 + 2 = 77 artifacts, which is the 3 epics and 73 stories plus the project file.
- Denylist: the decision and its justification are in `story:rules-sanitization-gate`, and this repository holds no list, hash or count of the private names.
- Import mechanics: fresh tree, seven category pull requests, and a provenance digest per rule are claimed once by `story:rules-import-mechanics`, with the per-category content in `story:rules-sanitize-and-fixtures`.
- Hunts runner, climb loop in Rust, and what stays private: the runner and the climb lane cover them with no Python.
- Docs generated from the rule files: `story:rules-catalogue-docs`.
- License and attribution: `story:rules-third-party-attribution`.
- Retiring the companion and re-pointing its consumers: `story:codesec-retirement`.
- I found no reach beyond the parent and no two items claiming one outcome.

What I read: 13 artifacts (the epic, its 11 stories, and `review-result:srules-scope-round-1`) with `aep plan artifact show`, plus `aep plan artifact graph`, `aep plan artifact validate`, and `ls` and `grep` over `.engineering/planning/`. I extracted 17 promises, the same list as round 1. I traced 16 to an item, with the E7 cell closed as the epic's stated owner decision. The unclaimed "2 scripts" row of the inventory table is the finding above and was not one of the 17.

What I could not establish:
- Which two files the "2 scripts" are, because the file list is private. The finding rests on no story's scope or body claiming a script move.
- Whether the "guides" in the rewrite row are all covered. They are spread across `docs/rules.md` sections and the site page, and no body says so.
- Out of my lane, did not set the verdict:
  - `story:rules-sanitize-and-fixtures` says the 32 existing fixtures are "checked", while the epic's table says 103 fixture files "move as is" and the epic requires fixtures "written from scratch". That is a design question about how the two statements fit.
  - `story:rules-planning-merge` says a codespec story is drafted where one of the 21 mirrored stories names none, but its scope and Done When list only two new codespec stories.
  - `aep plan artifact validate` reports two unrelated review-result records with no recorded outcome.

```findings
- file: .engineering/planning/story/rules-climb-lane.md
  line: 87
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the title and summary promise that the reader corpus loop runs the rules, but the body gives only the summary.json interface and defers the loop's rule step to an epic not in this store, and neither the epic's Out of scope nor the story's Done When records that part as not covered (retitle, or record it as not covered in the epic and the story)"
- file: .engineering/planning/epic/security-rules-in-codespec.md
  line: 47
  category: scope
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "the inventory table says 2 scripts move as is, but no story claims moving them: story:rules-runner replaces the hunt task and a shell script with a Rust command, story:rules-import-mechanics adds new scripts, and no scope lists a moved script (name the story that takes them, or change the row to rewrite or drop)"
```
