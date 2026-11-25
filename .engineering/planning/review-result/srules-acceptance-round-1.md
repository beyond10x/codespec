---
format: aep.planning-md/3
id: review-result:srules-acceptance-round-1
kind: review-result
status: active
title: Plan critic acceptance, round 1, epic security-rules-in-codespec
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
story:rules-import-mechanics — two Done When bullets ("`git log --format=%an -- rules/` on `main` shows only the bot", "after the last import, `codespec rules list` counts the rules the seven pull requests list") can only be true after story:rules-sanitize-and-fixtures, which depends_on this story, so this story cannot honestly close; move them to that story or reword them to this story's own script test — .engineering/planning/story/rules-import-mechanics.md:58
story:rules-layout — the Outcome promises that the release archive carries `rules/`, that `codespec rules list --format json` prints the codespec version, and that `docs/rules.md` names the four overlapping rules, but no Done When bullet checks any of them (a story that adds none of the three still passes) — .engineering/planning/story/rules-layout.md:69
story:rules-runner — the Outcome fixes the names-list merge order ("the rule's defaults, the run's `module`, then one `<rule>.json` per rule"), but the only Done When bullet ("a `--names` file for one rule changes that rule's rows only") passes whatever the order is; no test pins it — .engineering/planning/story/rules-runner.md:45
story:rules-climb-lane — the Outcome has a gate part that runs only `codespec-climb ratchet`, a `climb.yml` slow lane, digested data files and a reader-corpus rule step "when that epic has landed", but the Done When checks only the crate's tests and a count comparison; the part in `scripts/check-parts.txt` and the workflow are not checked, and the conditional corpus-loop line cannot be observed at closure — .engineering/planning/story/rules-climb-lane.md:53
story:rules-catalogue-docs — the E7 outcome says "wherever that comparison is published, codespec's cell is read from `website/data/rules.json`", but no page in this repository holds the comparison (`git grep "maintained rule catalogue"` finds only this story), and no Done When bullet checks it, so it cannot be seen done or not done — .engineering/planning/story/rules-catalogue-docs.md:51
story:rules-catalogue-docs — "The status page's capability list gains the catalogue as `shipped`, naming its check" has no Done When bullet, so the transition from no entry to a shipped entry is unchecked — .engineering/planning/story/rules-catalogue-docs.md:50
story:rules-ui-crate — the Outcome says the UI "runs `codespec rules run` … instead of its own scan loop" and lists the pinned script in `rules/NOTICE`, but no Done When bullet checks either, and the security properties other than Origin and URL password (Host check, loopback bind, no hooks or LFS, redaction) are named only through "tests moved" — .engineering/planning/story/rules-ui-crate.md:38
story:codesec-retirement — the Outcome says the open codesec issues are closed, the skill's Python helpers are ported and `scripts/rules-import.sh` and its test are deleted, but the Done When has no bullet that shows any of these (a plain `test ! -e scripts/rules-import.sh` would) — .engineering/planning/story/codesec-retirement.md:44
story:rules-planning-merge — the Outcome table says each group of codesec artifacts is "rewritten as stories" and two new codespec stories are "drafted by this story", but the Done When only validates "the rewritten artifacts" and checks issue items, so zero rewritten or drafted artifacts passes; no bullet names the expected artifacts or their count — .engineering/planning/story/rules-planning-merge.md:57

What I read: 12 artifacts, all given ids (the epic and 11 stories), whole bodies, via `aep plan artifact show <id>`; I also ran `aep plan artifact kinds` and `lifecycle story`, and checked the tree with `git grep`, `ls` and `aep plan artifact show` on the stories planning-merge names (all exist; story:rust-reader-whole-crate is implemented).

What I could not establish: epic:reader-self-improvement-loop is not in this store (it is on another branch), so the corpus-loop interface of the climb story cannot be checked against it. Multi-bullet Done When sections match the repo's own convention (e.g. `.engineering/planning/story/check-not-applicable.md:51`), so I did not call them "more than one statement". The epic has no Done When section; I treated its stories as its acceptance. Overlap of the seven pull requests between import-mechanics and sanitize-and-fixtures, and the shared test file, belong to the design and parallel-safety critics and set no verdict here.

```findings
- file: .engineering/planning/story/rules-import-mechanics.md
  line: 58
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "two Done When bullets (git log author check on rules/, rules list count after the last import) can only be true after story:rules-sanitize-and-fixtures, which depends_on this story, so this story cannot close; move them or reword them to this story's own script test"
- file: .engineering/planning/story/rules-layout.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the release archive carries rules/, list --format json prints the codespec version, and docs/rules.md names the four overlapping rules, but no Done When bullet checks any of them"
- file: .engineering/planning/story/rules-runner.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome fixes the names-list merge order (defaults, run module, per-rule file) but the only Done When bullet passes whatever the order is; no test pins it"
- file: .engineering/planning/story/rules-climb-lane.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome's gate part running only codespec-climb ratchet, the climb.yml slow lane, digested data files and the conditional reader-corpus rule step have no Done When check; the corpus-loop line cannot be observed at closure"
- file: .engineering/planning/story/rules-catalogue-docs.md
  line: 51
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the E7 outcome reads 'wherever that comparison is published' but no page in this repository holds the comparison and no Done When bullet checks it, so it cannot be seen done or not done"
- file: .engineering/planning/story/rules-catalogue-docs.md
  line: 50
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the status page gaining the catalogue as shipped, naming its check, has no Done When bullet, so the transition from no entry to a shipped entry is unchecked"
- file: .engineering/planning/story/rules-ui-crate.md
  line: 38
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the UI runs codespec rules run instead of its own scan loop and lists the pinned script in rules/NOTICE, but no Done When bullet checks either; most guard properties are covered only by 'tests moved'"
- file: .engineering/planning/story/codesec-retirement.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome's issue closure, Python helper port and deletion of scripts/rules-import.sh and its test have no Done When bullet that shows them"
- file: .engineering/planning/story/rules-planning-merge.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Done When validates only 'the rewritten artifacts' and issue items, so zero rewritten or newly drafted artifacts passes; no bullet names the expected artifacts or count"
```
