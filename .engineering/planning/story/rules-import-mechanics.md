---
format: aep.planning-md/3
id: story:rules-import-mechanics
kind: story
status: draft
title: Import as a fresh tree, one pull request per category, a provenance line per rule
summary: No companion history; a POSIX sh import script per category writing provenance digests; seven pull requests smallest first.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-layout
- depends_on: story:rules-sanitization-gate
- depends_on: story:rules-third-party-attribution
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: scripts/rules-import-test.sh
- confidence: inferred
  path: scripts/rules-import.sh
- confidence: inferred
  path: scripts/testdata/rules-import
revision: 5
---
# Story: Import as a fresh tree, one pull request per category, a provenance line per rule

## Outcome

This story delivers the import tool and the rules of the import; the seven category pull requests
themselves, and what each holds, are `story:rules-sanitize-and-fixtures`.

- No codesec history enters this repository. The inventory of 2026-11-25 found its diffs carry
  review outputs of private and unreported runs; a filtered history would have to drop a directory,
  rewrite the planning store and re-author a commit, and gains nothing a provenance line does not
  give. Files are copied from a `git archive` of a named codesec commit into an export directory
  outside this repository, where each category is sanitized and given its fixtures before import.
- `scripts/rules-import.sh <export dir> <category>` (POSIX sh) copies one category's rule files and
  fixtures from the export directory into `rules/<category>/`, writes each rule's `provenance:
  imported from codesec sha256:<digest of the file as first exported>`, regenerates the fixture
  documents (`task rules-fixtures`) and runs `codespec rules check`; on a refusal it restores
  `rules/<category>/` and exits non-zero naming the file. It decides nothing about a rule's
  content. It is deleted by `story:codesec-retirement`.
- The order of the category pull requests, smallest review first: `inspect` (5 rules),
  `correctness` (8), `performance` (8), `resource` (8), `concurrency` (11), `security` (12),
  `robustness` (17); each carries only `rules/<category>/`, its `CHANGELOG.md` line and, where a
  rule overlaps a built-in check, its line in `docs/rules.md`.
- The content of codesec's two open pull requests (a runner with a language filter and bounds; two
  Rust rules with wider coverage) is taken from their branches once they are merged or closed
  there.

## Why

A single import of 69 rules and 103 fixture files cannot be reviewed for private content line by
line; seven reviews of 5 to 17 rules can. A fresh tree is the only form the inventory allows.

## Done When

Failing test first: `scripts/rules-import-test.sh` (run by `task rules-import-test`), on a made-up
export of two rules with fixtures under `scripts/testdata/rules-import/`; within 60 s:

- the script copies both and exits 0; each written `provenance` digest equals `sha256sum` of the
  exported file;
- with one rule whose `category` field is wrong it exits non-zero naming the file, and
  `git status --porcelain rules/` is empty afterwards;
- with one rule without a fixture it exits non-zero naming the rule (the check of
  `story:rules-layout`);
- the test is red before the script exists.

## Scope

`scripts/rules-import.sh`, `scripts/rules-import-test.sh`, `scripts/testdata/rules-import/`,
`Taskfile.yml`. Depends on `story:rules-layout`, `story:rules-sanitization-gate` (no rule lands
before the gate) and `story:rules-third-party-attribution` (no rule lands without its provenance
check).
