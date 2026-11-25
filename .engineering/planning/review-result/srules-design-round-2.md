---
format: aep.planning-md/3
id: review-result:srules-design-round-2
kind: review-result
status: active
title: Plan critic design, round 2, epic security-rules-in-codespec
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
approve

Both round 1 findings are fixed, and the new depends_on edges introduce no new defect.

- story:rules-import-mechanics (round 1, finding 1) is fixed at revision 5. It now holds only the script, its test and the import rules. Its Done When no longer reads `rules/` after the seven pull requests, and it says the seven pull requests "are `story:rules-sanitize-and-fixtures`". The reversed read is gone.
- story:rules-sanitize-and-fixtures (round 1, finding 2) is fixed at revision 5. It says "No change to `crates/rules` here". The `fixture: null`, missing near-miss and overlap-table refusals now sit in story:rules-layout's outcome and Done When, which the import script reaches through the existing edge. The tool-versus-content seam is now clean.

New edges checked:
- story:rules-import-mechanics depends on story:rules-sanitization-gate and story:rules-third-party-attribution.
- story:rules-sanitize-and-fixtures depends on story:rules-runner.
- story:rules-climb-lane, story:rules-ui-crate and story:rules-catalogue-docs depend on one another in a line.
- story:codesec-retirement depends on five stories.

No acceptance line reads state from a sibling that has no edge, directly or through a chain. The chain from story:rules-climb-lane to story:rules-ui-crate to story:rules-catalogue-docs is not a serialising chain. Each edge records its reason beside it, namely the shared workspace files, `AGENTS.md`, `CHANGELOG.md`, `crates.md` and `scripts/check-parts.txt`. Both ends of each edge say the same thing.

What I read: 12 artifacts and review-result:srules-design-round-1, with `aep plan artifact show` (twice, full bodies), `relations`, `graph` and `validate`. Validate printed "valid", plus older review-result housekeeping that is not a finding.

Graph walk:
- I walked 25 `depends_on` edges and 11 `decomposes` edges. The only target outside the set is the epic.
- The graph is acyclic. story:rules-layout is the single root. The edges fan out, rejoin, and end at story:codesec-retirement.
- I traced about 20 acceptance reads to a producer in the set. All have an edge, direct or transitive. The reads include:
  - `rules/NOTICE` read by story:rules-ui-crate.
  - `summary.json` read by story:rules-climb-lane.
  - `rules list --format json` read by story:rules-catalogue-docs.
  - the import script and its test read by story:codesec-retirement.
  - the overlap table, written by story:rules-sanitize-and-fixtures and read by story:rules-layout's check.

What I could not establish:
- Not a defect: `epic:reader-self-improvement-loop` is cited by story:rules-climb-lane and has no artifact in this store. It is a dependency outside the set.
- Not a defect: story:rules-layout names `provenance` in its field list and cites story:rules-third-party-attribution for its values. Layout only checks that the field is present, so no read of produced state results.
- Out of my lane (acceptance): story:rules-sanitization-gate states that every repository URL in `rules/climb/` must be in the public-repository allow list. No Done When line tests it.
- Out of my lane (acceptance): in story:rules-climb-lane, the line about adding the rule step to the reader corpus loop's lane has no checkable acceptance. This is carried over from round 1.
- Out of my lane (parallel safety): the stories that share `crates/rules/tests/main.rs`, `AGENTS.md` and `CHANGELOG.md`.
- Out of my lane (scope): story:rules-planning-merge maps issue 14's "one file per module" item to `story:rust-reader-whole-crate`, which is already implemented.

```findings
[]
```
