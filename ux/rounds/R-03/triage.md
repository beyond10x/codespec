# R-03 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md`, `round.json`, `review-heuristics.md`
and `review-report.md` of R-03.

## Caveats

- No L6 sessions, no questionnaire, no L4/L5. Findings come from the heuristic review (screenshots,
  Playwright probes, `file:line`) and the report review; the every-merge harness found none.
- `report.md` labels the round `1988d91`; the tree measured was `1988d91` plus an uncommitted prose
  pass over 11 pages (`review-report.md` T2).
- No round metric sees structure or amount of content (`review-report.md` § 2). The change set's
  targets are therefore the regression assets in `ux/tests/l2-findings.spec.ts` and
  `crates/docs/src/status.rs`, each red before its change.
- Frequency is scenarios affected ÷ the 27 scenarios of the round (21 pages, 6 task cards).
- Confidence 2 where both reviewers saw the problem: UXF-006 (stale release on the home page) and
  UXF-012 (the mobile header: the overlap in one review, the axe target-size hit on the menu button
  at 768x1024 and 390x844 in the other).

## Did the last change sets move their metrics?

| round | change | target metric | moved |
|---|---|---|---|
| R-01 | UXF-001 search | `always_visible.misses` 224, `keyboard.misses` 76 | 0 and 8 in R-02 and R-03 (8 = UXF-005, counted twice: T1) |
| R-01 | UXF-002 underlined links | `axe.serious` 180 | 0 |
| R-01 | UXF-003 sidebar in view at the footer | `always_visible.misses` sidebar part 72 | 0 |
| R-01 | UXF-004 legend by keyboard | `axe.serious` on the support page 2 | 0 |

## Formula

| finding | severity | weight | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-012 phone search covers the menu button | S2 | 3 | 21/27 = 0.78 | 2 | S = 1 | **4.67** |
| UXF-007 home: no action in the first screen, Status repeated | S2 | 3 | 7/27 = 0.26 | 1 | S = 1 | **0.78** |
| UXF-010 sidebar: 24 items open, active item below its fold | S3 | 1 | 19/27 = 0.70 | 1 | S = 1 | **0.70** |
| UXF-014 code and outline under 14 px | S3 | 1 | 19/27 = 0.70 | 1 | S = 1 | **0.70** |
| UXF-006 three releases, two language lists | S2 | 3 | 6/27 = 0.22 | 2 | M = 2 | **0.67** |
| UXF-008 static analysis on four pages | S2 | 3 | 7/27 = 0.26 | 1 | M = 2 | 0.39 |
| UXF-009 overview repeated on five pages | S3 | 1 | 5/27 = 0.19 | 1 | S = 1 | 0.19 |
| UXF-011 up to five names per page | S3 | 1 | 6/27 = 0.22 | 1 | M = 2 | 0.11 |
| UXF-013 generated pages print whole documents | S3 | 1 | 5/27 = 0.19 | 1 | M = 2 | 0.09 |
| UXF-005 search page focuses its box on load | S3 | 1 | 1/27 = 0.04 | 1 | M = 2 | 0.02 |

No S1 finding. The five highest fill the set; UXF-008, 009, 011 and 013 (the content reduction the
operator asked for) are the next set, R-04.

## Change set R-03

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-012 | `website/src/css/custom.css`: below 997px the search container sits left of the menu button | toggle centre covered by `input.navbar__search-input` → uncovered at 390x844 and 768x1024 | `ux/tests/l2-findings.spec.ts::UXF-012` |
| 2 | UXF-007 | hero copy aligned to the top; lede 80 → 45 words; status section and KPI row removed | "Get started" bottom 955 px / 949 px → in the first screen at 1366x768 / 1280x720; words in main 1311 → ≤ 700 | `ux/tests/l2-findings.spec.ts::UXF-007` |
| 3 | UXF-010 | `website/sidebars.ts`: Start here (overview, Getting started), Guides, Concepts, Examples, Reference, Project; collapsed groups, `autoCollapseCategories` | active item at 808 px and 909 px → in the sidebar's first screen | `ux/tests/l2-findings.spec.ts::UXF-010` |
| 4 | UXF-014 | `custom.css`: code blocks 0.9rem, outline 0.875rem | 13.12 px / 12.8 px → ≥ 14 px | `ux/tests/l2-findings.spec.ts::UXF-014` |
| 5 | UXF-006 | `product.json` and `status.mdx` name no release; Getting started pastes 0.6.0 and the ten adapters; the index lists four readers | a release other than the workspace version on 3 pages → 0 | `crates/docs/src/status.rs::the_site_states_only_the_workspace_version` |

No change needs a design decision (§ 3.4): each keeps the site's components and moves or removes
content.
