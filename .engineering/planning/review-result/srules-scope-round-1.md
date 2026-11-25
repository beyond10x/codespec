---
format: aep.planning-md/3
id: review-result:srules-scope-round-1
kind: review-result
status: active
title: Plan critic scope, round 1, epic security-rules-in-codespec
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
.engineering/planning/epic/security-rules-in-codespec.md — the epic promises "sanitize | 18 (… 13 measurement data and prose files)" and no story claims sanitizing them: story:rules-climb-lane only says its data "moves to rules/climb/ with the attribution of rules/NOTICE", and the structural checks of story:rules-sanitization-gate cover rule text and fixtures only (climb-lane should take it, with a Done When) — .engineering/planning/epic/security-rules-in-codespec.md:48
story:rules-catalogue-docs — the owner's "E7 cell counting the public catalogue" is narrowed to "wherever that comparison is published": the comparison is not in this tree (git grep finds no E7 comparison cell outside unrelated review comments), and the scope names no file where the cell lives, so the promise can be marked done while landing nowhere (name the file or the repository and its change, or record the part as not covered) — .engineering/planning/story/rules-catalogue-docs.md:51

What I read: 12 artifacts (the epic and its 11 stories) with `aep plan artifact show`, plus `aep plan artifact graph`; I also checked by `ls`/`git grep` that the codespec stories named in the issue 13 and 14 map exist, and that `epic:reader-self-improvement-loop` is not in this store (the climb-lane names it as planned on its own branch, so I did not count it a gap). I extracted 17 promises from the epic and the owner's request, listed here so you can check my reading.

1. Layout and versioning.
2. `codespec rules` lists, checks and runs the catalogue.
3. Rules ship with each release.
4. A sanitization gate in CI forever.
5. The denylist is kept without publishing it, decided and justified.
6. From-scratch fixtures with generic names.
7. Import mechanics: fresh tree or filtered history, one PR per category, provenance per rule.
8. Rule-by-rule sanitization with positive and near-miss rows.
9. The hunts runner.
10. The climb loop, in Rust, with no Python.
11. What stays private.
12. The public reader corpus loop runs the rules.
13. Docs generated from the rule files.
14. E7 counts the public catalogue.
15. Planning merge with issues 13 and 14 mapped.
16. Retirement: archive, consumers re-pointed, private outputs kept private.
17. License and attribution of third-party material.

I traced 15 of the 17 to an item. Promise 14 is only partly traced (finding 2). The measurement-data sanitization row of the inventory table, which is not in the 17, is finding 1. I found no reach beyond the parent and no two items claiming one outcome. The seven category PRs, the D1 to D5 decisions and the story order all match the epic.

What I could not establish:
- Whether the second inventory row, "1 doc", sanitized, and the "guides" in the rewrite row have a home: the parent does not name them, so I raised no finding.
- The Datastar line in `rules/NOTICE` is stated in `story:rules-third-party-attribution` but landed by `story:rules-ui-crate`, and neither Done When checks it. That is acceptance and design, not scope, so it is out of my lane and did not set the verdict.

```findings
- file: .engineering/planning/epic/security-rules-in-codespec.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises to sanitize 13 measurement data and prose files and no story claims it: story:rules-climb-lane only says its data moves to rules/climb/ with the attribution of rules/NOTICE, and the structural checks of story:rules-sanitization-gate cover rule text and fixtures only (climb-lane should take it, with a Done When)"
- file: .engineering/planning/story/rules-catalogue-docs.md
  line: 51
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the owner's E7 cell counting the public catalogue is narrowed to 'wherever that comparison is published': the comparison is not in this tree and the scope names no file where the cell lives, so the promise can be marked done while landing nowhere (name the file or repository and its change, or record the part as not covered)"
```
