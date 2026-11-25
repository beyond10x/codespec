---
format: aep.planning-md/3
id: epic:security-rules-in-codespec
kind: epic
status: draft
title: The security rule catalogue lives in codespec, public and sanitized
summary: Move the cq rule catalogue, its fixtures, runner, measurement loop and UI from the private companion repository into codespec under rules/, behind a sanitization gate, and archive the companion.
revision: 3
---
# Epic: The security rule catalogue lives in codespec, public and sanitized

## Outcome

The cq rule catalogue that grew in the private companion repository codesec (69 rule files in seven
categories, 32 fixtures, a runner, a measurement loop over published Go security fixes and a local
catalogue UI) lives in this repository, public, under `rules/`, and ships with every codespec
release. `codespec rules` lists, checks and runs it. Every rule and every fixture in this
repository is free of any private name: a sanitization gate runs in CI on every pull request and
push, forever, and every fixture is made-up code written from scratch. codesec is archived after
the move; what stays private is only what is private by nature: runs over a private code base and
the review outputs of such runs.

The split ends: one repository holds the engine and the rules written for it, a rule that needs a
new fact is a story beside the reader that would give it, and the catalogue is counted, documented
and released like every other part of codespec.

## Why Now

The owner, 2026-11-25: "I don't want to have that split, and I want it to be public. There is no
reason to hide anything here. obviously all the queries and pattern matches in codesec need to be
like sanitized, right, and not carry any project specific and company related stuff."

The split costs today: a rule that needs a reader change is planned twice (codesec holds 21
`needed-from-codespec-*` stories that mirror codespec stories); codesec pins a codespec release and
lags it; the public site says the catalogue is private (`website/docs/codesec.md`), so the
comparison of this tool with others counts a catalogue nobody can see (E7, "ships a maintained rule
catalogue": partial); and the rules are never run by codespec's own gate, so a reader change that
breaks a rule is found in a private run, late.

## What the inventory found (counts only; the file list is private)

An inventory of codesec `main` and its two open pull requests was taken on 2026-11-25 outside every
repository. Its result, as counts:

| action | files |
|---|---|
| move as is, with a provenance line | 205 (66 rule files, 2 catalogue lists, 103 fixture files, 34 UI files) |
| sanitize | 18 (3 rule descriptions and 1 doc: `story:rules-sanitize-and-fixtures`; 1 UI placeholder: `story:rules-ui-crate`; 13 measurement data and prose files: `story:rules-climb-lane`) |
| rewrite | 21 (the measurement loop's 8 Go and 5 shell files, 2 Rust-repository scan scripts replaced by `codespec rules run` in `story:rules-runner`, the Taskfile, the guides, a skill) |
| drop | 3 |
| stays private | 16 (every review and sweep output, and the list of reviewed code bases) |

37 of the 69 rules have no fixture. The git history of codesec cannot be published: its diffs carry
the review outputs. The import is a fresh tree.

## Decisions this epic takes by default (the owner may change each)

- D1 Denylist: the existing Gates policy (`forbidden_literals`, `forbidden_patterns`), held in a
  private repository and given to CI as a secret, carries the private names; this repository
  commits no list and no hash of one (`story:rules-sanitization-gate`).
- D2 History: fresh-tree import, no codesec history (`story:rules-import-mechanics`).
- D3 The measurement loop is ported to Rust and runs as a separate slow lane
  (`story:rules-climb-lane`).
- D4 The catalogue UI moves as its own crate, last (`story:rules-ui-crate`).
- D5 codesec is archived, not deleted; review outputs of private code go to a private repository
  the owner names (`story:codesec-retirement`).

Open, not taken by this epic: whether the comparison of codespec with other tools is published on
this site. `story:rules-catalogue-docs` makes `website/data/rules.json` the one source of the E7
count either way.

## Stories

1. `story:rules-layout`: `rules/<category>/<id>.yaml`, fixtures, `crates/rules`, `codespec rules
   list|check`, versioned with the release.
2. `story:rules-sanitization-gate`: the denylist in CI plus structural checks on every rule and
   fixture.
3. `story:rules-third-party-attribution`: `rules/NOTICE`, per-rule `provenance` and `references`.
4. `story:rules-import-mechanics`: fresh tree, one pull request per category, provenance per rule.
5. `story:rules-sanitize-and-fixtures`: the seven category imports, each rule generalized or
   dropped, each kept rule with a from-scratch fixture holding positive and near-miss rows.
6. `story:rules-runner`: `codespec rules run` with a language filter, per-rule bounds, batches and
   test-file exclusion; what stays private.
7. `story:rules-climb-lane`: the measurement loop over published Go fixes, in Rust, as a slow lane;
   its data sanitized; the runner summary the reader corpus loop reads.
8. `story:rules-catalogue-docs`: the catalogue on the site, generated from the rule files; the E7
   count's one source.
9. `story:rules-planning-merge`: which codesec artifacts carry over; its open issues mapped.
10. `story:rules-ui-crate`: the catalogue UI as a crate.
11. `story:codesec-retirement`: archive codesec and re-point its consumers.

Order: 1, 2, 3, 6 one after another (they share `crates/rules`, its test file and
`docs/rules.md`); 9 beside them after 2; 4 after 3; 5 after 4 and 6; then 7, 10, 8 one after
another (they share the workspace files, `AGENTS.md` and the generated crate page); 11 last.

## Out of scope

- The rule step inside the reader corpus loop's lane: a story of `epic:reader-self-improvement-loop`,
  which is planned on its own branch. This epic gives it the interface (`codespec rules run`'s
  `summary.json`, `story:rules-runner`) and does not add the step.
- New rules. The Java rules planned in codesec carry over as plans (`story:rules-planning-merge`)
  and are written later under this repository's process.
- Any run over a private code base, and any finding of one. Neither is committed here.
- Reader and cq changes a rule waits for: each is its own story beside the reader.
- The branch ruleset, required checks and the Gates policy repository's own review: the owner's.
