# R-08 operator notes

- Goal of the loop: improve the UI of the public docs site. This run is up to three rounds (R-08,
  R-09, R-10), and stops early when a round finds nothing new above the triage threshold.
- Scope of this run: the site UI only. Another branch edits the prose of `website/docs/**/*.md` at
  the same time, so no change set of this run edits the text of a page under `website/docs/`.
  Changes go to `website/src/**`, CSS, theme wrappers, `docusaurus.config.ts`, `sidebars.ts`,
  static assets, and the harness in `ux/`. A finding whose fix is page text stays open with that
  reason; a finding whose fix belongs in `@beyond10x/docs-system` is reported upstream, not patched
  in `node_modules`.
- The tree measured is `main` at the commit in `report.md` (`0.11.0` plus 2 commits), unchanged.
  Since R-07's change set the releases added the Evaluator column to the support matrix, the
  codesec page and changes to six docs pages.
- The run wrote its output to `R-09` (T15: `nextRound` counted this directory, which held only
  this file). The output was moved here and `report.md`, `round.json` and the trend row rewritten
  with `writeRound` from the same results. Three `page-docs` gating units (1366x768 light and dark,
  1280x720 dark) timed out at 60 s under load (T16, T17); the report reviewer reran them alone
  (passed, about 4 s each) and their result files were added before the rewrite, so the report
  counts 96 of 96 gating units.
- Queue carried from R-07, in priority order: UXF-028 (page text: out of scope here), UXF-005,
  T1 (keyboard misses counted twice), UXF-019 (page text: out of scope here), UXF-021, T3
  (scroll-container allowlist misses `.b10x-table-wrap`), T2 (bare commit over a dirty tree).
- No L6 sessions, no questionnaire, no L4/L5/L8: no paid APIs and no people in this loop.
- The machine is shared and loaded: the round runs with 2 Playwright workers.
