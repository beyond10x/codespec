# R-01 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md` and `round.json` of R-01.

## Caveats

- No L6 sessions, no questionnaire, no L4/L5: every finding comes from the every-merge harness (L2, L3).
- The two review agents (§ 8) left no `review-heuristics.md` or `review-report.md` in R-01, so no
  finding has a heuristic review as a second source.
- Frequency is pages affected ÷ the 19 page scenarios R-01 checked at the gating viewports.
  The netcheck page (`page-docs-examples-netcheck`) joined after R-01 and is not in these counts.
- Confidence 2 only for UXF-001: L2 (gate 2 and gate 5 keyboard) and L3 (the screen-only user takes
  10 actions on the two CLI tasks for want of search) are two separate instruments that agree.

## Did the last change sets move their metrics?

| round | change | target metric | moved |
|---|---|---|---|
| – | none: R-01 is the first round | – | – |

## Formula

```
priority = severity weight × frequency × confidence ÷ cost
severity weight  S1 = 9, S2 = 3, S3 = 1
cost             S = 1, M = 2, L = 4
```

| finding | severity | weight | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-001 no search | S2 | 3 | 19/19 = 1.00 | 2 | M (dependency, navbar, bot) = 2 | 3 × 1.00 × 2 ÷ 2 = **3.00** |
| UXF-002 links by colour only | S2 | 3 | 13/19 = 0.68 | 1 | S (CSS) = 1 | 3 × 0.68 × 1 ÷ 1 = **2.05** |
| UXF-003 sidebar scrolls out at footer | S3 | 1 | 18/19 = 0.95 | 1 | S (CSS) = 1 | 1 × 0.95 × 1 ÷ 1 = **0.95** |
| UXF-004 legend not keyboard-reachable | S2 | 3 | 1/19 = 0.05 | 1 | S (component) = 1 | 3 × 0.05 × 1 ÷ 1 = **0.16** |

No S1 finding. Four findings, under the cap of 5 changes.

## Change set R-01

| # | finding | change | target metric (R-01 → expected) | regression asset |
|---|---|---|---|---|
| 1 | UXF-001 | local, offline search in the navbar | `always_visible.misses` 224 → 72 (search part 76 → 0, plus 76 at the bottom state is sidebar); `keyboard.misses` 76 → 0; screen-only actions on task-trace-options and task-reach-reports 10 → fewer | `ux/tests/l2-layout.spec.ts::L2` (always-visible search, keyboard tab-reaches-search), `ux/tests/l3-screen-only.spec.ts::L3` |
| 2 | UXF-002 | underline links in running text, both themes | `axe.serious` 180 → 2 (link-in-text-block 0) | `ux/tests/l2-layout.spec.ts::L2` (axe) |
| 3 | UXF-003 | sticky sidebar with its own scroll, in view at the footer | `always_visible.misses` sidebar part 72 → 0 | `ux/tests/l2-layout.spec.ts::L2` (sidebar at the bottom state) |
| 4 | UXF-004 | the legend's scroll box focusable, a named region | `axe.serious` support page 2 → 0 | `ux/tests/l2-layout.spec.ts::L2` (axe on page-docs-reference-support) |

No change needs a design decision (§ 3.4): UXF-003 keeps Docusaurus's own sticky sidebar and only
lets it stay in view; it is dropped with a written reason if it breaks gate 1.
