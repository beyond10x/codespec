---
format: aep.planning-md/3
id: story:codesec-retirement
kind: story
status: draft
title: codesec is archived and every consumer reads the rules from codespec
summary: Re-point the security-review skill and other consumers, keep private review outputs in a private repository, close the companion's pull requests and issues, archive it.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-sanitize-and-fixtures
- depends_on: story:rules-climb-lane
- depends_on: story:rules-catalogue-docs
- depends_on: story:rules-planning-merge
- depends_on: story:rules-ui-crate
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: scripts/rules-import-test.sh
- confidence: inferred
  path: scripts/rules-import.sh
- confidence: inferred
  path: scripts/testdata/rules-import
revision: 4
---
# Story: codesec is archived and every consumer reads the rules from codespec

## Outcome

- Consumers re-pointed: the security-review skill reads `rules/` of a codespec checkout or release
  archive instead of a codesec checkout, runs `codespec rules run` instead of its own runner, and
  reads `docs/rules.md` and the generated rules page here; the skill's own helper scripts that are
  Python are ported (Rust, POSIX sh, or replaced by `codespec rules run`) under the rule that no
  permanent tool is Python. Every other session, task or script that names codesec is found by a
  search of the owner's machine and re-pointed.
- Private outputs: a review of a private code base, its names lists, rows, findings and reproducers
  go to a private repository the owner names; the skill says so and writes nothing of them into
  codespec. The review outputs already in codesec stay there, read-only.
- codesec's two open pull requests are merged or closed there first, and their content is in
  codespec (`story:rules-runner`, `story:rules-sanitize-and-fixtures`); its two open issues are
  closed with a link to the stories of `story:rules-planning-merge`.
- `scripts/rules-import.sh` and its test are deleted.
- codesec is archived on GitHub (read-only, still private), after the owner's word; it is not
  deleted.

## Why

Two copies of a rule drift. The owner wants one place; archiving keeps the private review record
without a second live catalogue.

## Done When

- A search of the skill and the owner's working directories for `codesec` finds only the archived
  repository's own files and history notes (the search and its output are the evidence);
- the skill's procedure runs end to end on the public example module `examples/netcheck` with
  `rules/` of this repository and writes its outputs outside it; it fails first when pointed at a
  codesec checkout path that no longer exists;
- `gh repo view beyond10x/codesec --json isArchived,visibility` shows archived and private;
- no file of this repository names a codesec path other than in `provenance` lines and
  `CHANGELOG.md`;
- `test ! -e scripts/rules-import.sh && test ! -e scripts/rules-import-test.sh &&
  test ! -e scripts/testdata/rules-import`, and `task -n rules-import-test` fails;
- `gh issue view 13 -R beyond10x/codesec --json state` and the same for 14 show `CLOSED`, each with
  a comment naming the codespec story ids;
- `find <the skill's directory> -name '*.py'` prints nothing;
- codesec's pull requests 15 and 16 show merged or closed.

## Scope

`scripts/rules-import.sh`, `scripts/rules-import-test.sh`, `scripts/testdata/rules-import/`
(removed), `Taskfile.yml`, `AGENTS.md`,
`CHANGELOG.md`. Outside this repository: the security-review skill, the codesec repository
(archive, owner's word). Depends on every other story of the epic.
