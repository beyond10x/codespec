---
format: aep.planning-md/3
id: story:rules-catalogue-docs
kind: story
status: draft
title: The rule catalogue on the public site, generated from the rule files
summary: codespec-docs generates the rules reference and counts; the private-companion page goes; E7 reads its count from the generated file.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-sanitize-and-fixtures
- depends_on: story:rules-ui-crate
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/docs/src
- confidence: inferred
  path: crates/docs/src/status.rs
- confidence: inferred
  path: ux/baseline.json
- confidence: inferred
  path: ux/scenarios
- confidence: inferred
  path: website/data/rules.json
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs/codesec.md
- confidence: inferred
  path: website/docs/reference/rules.md
- confidence: inferred
  path: website/docs/rules.md
- confidence: inferred
  path: website/docs/use-cases.md
- confidence: inferred
  path: website/sidebars.ts
revision: 4
---
# Story: The rule catalogue on the public site, generated from the rule files

## Outcome

- `codespec-docs` generates `website/docs/reference/rules.md` (every rule by category: id, title,
  description, languages, severity, status, `needs` for a pending rule, references, and its
  fixture's positive and near-miss rows) and `website/data/rules.json` (counts per category,
  language and status, and the codespec version) from `rules/`. `task docs-generate` writes them;
  `docs-check` fails when they are not current. They join the table of generated files in
  `AGENTS.md`.
- `website/docs/codesec.md` ("a private companion … what stays private") is removed; a page on the
  rules (`website/docs/rules.md`: what a rule is, how to run it, how to write one with a fixture)
  takes its place in the sidebar, and `website/docs/use-cases.md` points at it. The UX scenario of
  the old page is replaced and the UX baseline updated in the same change.
- The status page's capability list (`crates/docs/src/status.rs`) gains the rule catalogue as
  `shipped`, naming its check (`crates/rules/tests/main.rs`, the fixture check).
- E7, "ships a maintained rule catalogue", in the comparison of codespec with other tools: the
  comparison is not published in this repository today. `website/data/rules.json`'s count of active
  rules is the one source for codespec's cell; the comparison, wherever the owner publishes it,
  cites that file and that count. Publishing the comparison on this site is not this story (an
  open decision of the owner; when taken, it is its own story that reads this file).

## Why

A public catalogue nobody can browse is half public. The comparison counted a private library;
after the move it counts what anybody can read.

## Done When

Failing tests first, in `crates/docs` (its existing test target); within 60 s each:

- `every_generated_file_is_current_in_this_tree` fails when a rule file changes and the page was not
  regenerated, and passes after `task docs-generate`;
- the counts in `website/data/rules.json` equal `codespec rules list --format json` over the same
  tree, active rules counted under the key the comparison cites (`active`);
- the generated page holds no rule's `provenance` digest and no fixture source beyond its rows;
- `every_shipped_capability_rests_on_a_test_that_exists` passes with the new `shipped` entry and
  fails when the named check is renamed; `website/data/status.json` holds the entry;
- `website/docs/codesec.md` is gone and no page links to `/docs/codesec`;
- `task ux-check` passes with the new page in its scenarios.

## Scope

`crates/docs/src/` (a rules module and `status.rs`), `website/docs/reference/rules.md`,
`website/data/rules.json`, `website/docs/rules.md`, `website/docs/codesec.md` (removed),
`website/docs/use-cases.md`, `website/sidebars.ts`, `ux/scenarios/`, `ux/baseline.json`,
`website/data/status.json`, `AGENTS.md` (generated-files table). Depends on
`story:rules-sanitize-and-fixtures` and `story:rules-ui-crate` (shared `AGENTS.md`; lands after
`story:rules-climb-lane` and `story:rules-ui-crate`).
