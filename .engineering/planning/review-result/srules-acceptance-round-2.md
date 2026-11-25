---
format: aep.planning-md/3
id: review-result:srules-acceptance-round-2
kind: review-result
status: active
title: Plan critic acceptance, round 2, epic security-rules-in-codespec
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
story:rules-sanitize-and-fixtures — the Done When bullet "`git log --format=%an -- rules/` on `main` shows only the bot" names no bot anywhere in the set, and `rules/` already holds `categories.tsv`, `subcategories.yaml` and `NOTICE`, plus the `rules/climb/` data from later stories, so the bullet cannot give the same answer twice; scope it to `rules/<category>/` and name the author — .engineering/planning/story/rules-sanitize-and-fixtures.md:81

All nine round-1 findings are fixed:
- **`story:rules-import-mechanics`:** its Done When is now only its own script test, and the two bullets that needed the later story moved to `story:rules-sanitize-and-fixtures`. They landed there with the defect above.
- **`story:rules-layout`:** it now has bullets for `codespec_version`, the release-archive listing, the overlap table and `cli.md`.
- **`story:rules-runner`:** it now has a merge-order bullet with a case for each side.
- **`story:rules-climb-lane`:** the Outcome no longer promises the conditional corpus-loop step. The Done When now checks the climb part, the workflow triggers and the absence of `measure` from the gate, by grep.
- **`story:rules-catalogue-docs`:** the E7 outcome now says the comparison is not published here, and the cell's source is checked through the `active` count against `codespec rules list --format json`. The status entry has its own bullet.
- **`story:rules-ui-crate`:** bullets now cover Origin, Host and bind, URL password and hooks/LFS, redaction, the stub-binary run of `codespec rules run`, and the `rules/NOTICE` Datastar line.
- **`story:codesec-retirement`:** bullets now cover the issues, the `*.py` find, the `test ! -e` deletion check and `task -n`.
- **`story:rules-planning-merge`:** the Done When now names the three epics, the counts 8, 14 and 26 plus the dropped ones, and the two new stories. It also checks that each issue item names an artifact `aep plan artifact show` finds.

What I read: 12 artifacts (the epic and the 11 stories), whole bodies, via `aep plan artifact show <id>`, plus `aep plan artifact show review-result:srules-acceptance-round-1` and `grep` on the worktree's `.engineering/planning` for the line citation.

What I could not establish:
- Whether the corpus-loop interface matches `epic:reader-self-improvement-loop`; that epic is not in this store.
- Out of my lane and not counted in the verdict: `story:codesec-retirement` says "Depends on every other story of the epic" but its `depends_on` relations omit `story:rules-layout`, `story:rules-sanitization-gate`, `story:rules-third-party-attribution`, `story:rules-runner` and `story:rules-import-mechanics`. That is for the design critic.

```findings
- file: .engineering/planning/story/rules-sanitize-and-fixtures.md
  line: 81
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Done When bullet 'git log --format=%an -- rules/ on main shows only the bot' names no bot anywhere in the set, and rules/ already holds the layout story's lists, NOTICE and the climb data from other stories, so the check cannot give the same answer twice; scope it to rules/<category>/ and name the author"
```
