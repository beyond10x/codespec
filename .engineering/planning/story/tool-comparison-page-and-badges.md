---
format: aep.planning-md/3
id: story:tool-comparison-page-and-badges
kind: story
status: draft
title: The tool comparison as checked data, a page on the site and badges on the README
summary: codespec-comparison/1 data, a checker in codespec-docs, a generated site page and README badges counted from committed JSON.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/docs/src
- confidence: cited
  path: schemas/comparison-1.schema.json
- confidence: cited
  path: scripts/planning-only.sh
- confidence: cited
  path: website/data
- confidence: cited
  path: website/docs/comparison.md
- confidence: cited
  path: website/docusaurus.config.ts
- confidence: cited
  path: website/sidebars.ts
revision: 2
---
# Story: The tool comparison as checked data, a page on the site and badges on the README

## Outcome

The assessment of codespec against 21 other code analysis tools is `website/data/comparison.json`,
a `codespec-comparison/1` document: 30 dimensions (each a one-line question and the rule that
decides yes, partial or no), 22 tools (vendor, licence, date assessed), 660 cells (a value, a
one-line note and a source), the findings "where codespec stands out" and "where codespec is
behind", and the caveats. `schemas/comparison-1.schema.json` is generated from the Rust types the
file is read into. `codespec-docs` checks it, writes it back canonical and generates from it the
page `website/docs/comparison.md` (linked from the sidebar, the navigation, the footer and the
README), three shields endpoint files under `website/data/badges/` and the badge block at the top
of `README.md`.

## Why

The owner, 2026-11-25: "we want to really formalize all of that, also the assessment, and also
present these results on the public-facing website docs. And we also want to maybe add some fancy
badges on the README, the start page of the repo of CodeSpec. Yeah, integrate that with". The
comparison was a private Markdown file with one footnote per cell; a claim that rests on 660
footnotes needs a machine to hold the footnotes to their sources, and a claim of "only" needs a
check that no other column has the same verdict.

## Done When

- `cargo test -p codespec-docs` reads the data strictly into the types of its schema and refuses:
  a tool and dimension without exactly one cell; a cell whose value does not fit its dimension; a
  cell without a source (`unknown` excepted); a cited file of this tree that is missing or a line
  outside it; `planned` anywhere but a status cell of codespec, and any value but `planned` on a
  source that is on a branch other than `main`; a page that did not load cited as a source; a text
  that cites a cell or a line that is not there; a statement about who has `yes` that the cells
  do not hold. Each has a test that plants it.
- `codespec-docs comparison --check` and `generate --check` fail when the data is not canonical,
  or the page, the schema, a badge file or the README block differs from what the data renders.
  No network, no test over 5 s, the docs crate's tests in under 2 s.
- `codespec diff` (C1) is `planned` while the change that adds it (pull request 61) is not on
  `main`; merging it and releasing flips the cell to `yes` and its source to the file on `main`.
- Nothing private: no number or path of the companion rule library, no company, customer or
  person name.
- The README holds six badges, each linked to the page that backs it: release, `check` on `main`,
  licence, languages read and features verified (counted in `website/data/support-matrix.json`),
  and definite or possible answers (counted in the comparison: tools with `yes` on S3).

## Scope

- Own: `crates/docs/src/comparison.rs`, `comparison_page.rs`, `badges.rs`, `website/data/comparison.json`,
  `website/data/badges/`, `schemas/comparison-1.schema.json`, `website/docs/comparison.md`, the
  badge block of `README.md`, `scripts/planning-only.sh` (the root `README.md` is code now).
- The reader KPI badge belongs to `story:corpus-kpi-ratchet` (branch `plan/read-loop`, not on `main`
  yet), part of `epic:reader-self-improvement-loop`. This story leaves a slot for it in the badge
  block (`badges::SLOT`) and shows no number for it: that story adds the badge with its committed
  baseline, from the number its evidence file holds. Its body, on `plan/read-loop`, needs one line
  when that branch lands: this story owns the badge block and the slot. The relation is written
  here in prose because neither artifact is on `main` yet.
- The assessment is a snapshot of 2026-11-25 at one codespec revision. A new assessment is a pull
  request that changes the date, the revision and every cell it re-reads; it is not scheduled.
