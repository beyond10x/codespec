# R-08 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md`, `round.json`, `review-heuristics.md`
and `review-report.md` of R-08.

## Caveats

- No L6 sessions, no questionnaire, no recordings; no L4, L5 or L8: no paid APIs and no people in
  this loop. Findings come from the round's metrics and the two review agents.
- The measured tree is `24044631` (`0.11.0` plus 2 commits), not R-07's change set alone: the
  releases since added the Evaluator column to the support matrix, the codesec page and edits to six
  docs pages.
- The run first wrote R-09 (T15) and three `page-docs` units timed out at 60 s under load (T16,
  T17). The output was moved here, the three units rerun alone (passed, about 4 s each) and the
  report rewritten from the same results: 96 of 96 gating units.
- The ratchet is red on this tree: `page-docs-reference-support layout.overflow` 4 > 0, an
  unwaived gate 1 failure (UXF-034). Any merge has to fix it.
- Scope of this run: site UI and harness only. A branch editing the prose of `website/docs/` is
  open, so the page-text findings (UXF-032, UXF-033, UXF-019, UXF-028) are ranked and stay open for
  that work; none enters a change set of this run.
- Frequency is scenarios affected ÷ the 30 scenarios of the round (24 pages, 6 task cards).
- Confidence 2 where two sources agree: UXF-034 (the gate and both reviews), UXF-021 (the heuristic
  review and the report review's clipped count), UXF-005 (the waived gate failure and the heuristic
  review), UXF-036 (the report review and the raw results).

## Did the last change sets move their metrics?

| round | change | target | moved |
|---|---|---|---|
| R-07 | UXF-029 table code breaks anywhere | gating scroll containers 24 → 8 | yes: 8, all on the support matrix; the five pages have none; asset green |
| R-07 | UXF-027 Status describes the tree | Status phrases tying shipped to the release 3 → 0 | held in its Rust asset (outside the round, T12); the sentence it wrote now points to an empty Unreleased section after later releases (UXF-032, page text) |
| R-07 | UXF-025 home names the C writer | home places missing the C writer 3 → 0 | yes: `product.json` lede, description and write step name C |
| R-07 | UXF-031 scroll containers per page | pages named 0 → every page | yes: the per-page table exists (support, 8) |
| R-07 | UXF-030 matrix and Status data 14 px | ids, descriptions, code, headers, chips → 14 px | yes, asset green; with the Evaluator column it widened the matrix (848 → 960 px) and cut-off descriptions went 111 → 141 items (UXF-021) |

## Formula

| finding | severity | weight | frequency | confidence | cost | priority | fix in |
|---|---|---|---|---|---|---|---|
| UXF-032 Status points to an empty Unreleased section | S2 | 3 | 1/30 = 0.033 | 1 | S = 1 | 0.10 | page text: not this run |
| UXF-034 coverage bar label past its column (gate 1, unwaived) | S3 | 1 | 1/30 = 0.033 | 2 | S = 1 | **0.067** | UI |
| UXF-005 search page focuses its box | S3 | 1 | 1/30 = 0.033 | 2 | S = 1 | **0.067** | UI |
| UXF-035 pipeline status chips 10.4 px | S3 | 1 | 2/30 = 0.067 | 1 | S = 1 | **0.067** | UI |
| UXF-036 (T8, T16, T19) the round hides missing units and a red ratchet | S3 | 1 | 1/30 = 0.033 | 2 | S = 1 | **0.067** | harness |
| UXF-021 matrix scrolls sideways, descriptions cut | S3 | 1 | 1/30 = 0.033 | 2 | M = 2 | **0.033** | UI (decision) |
| UXF-033 archive contents on Getting started | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | page text: not this run |
| T15 `nextRound` counts a notes-only directory | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | harness |
| T1 keyboard misses counted twice | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | harness |
| T17 one 60 s timeout in a loaded round | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | harness |
| T18 failure shot shows the page top for an overflow | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | harness |
| T3 allowlist misses `.b10x-table-wrap`, clipped unreported | S3 | 1 | 1/30 = 0.033 | 1 | S = 1 | 0.033 | harness |
| UXF-019, UXF-028 | S3 | 1 | 1/30 | 1 | S | 0.033 | page text: not this run |
| T2, T7, T10, T11, T13 (latent) | S3 | 1 | 0 | 1 | – | 0 | harness |

No S1 finding. The four at 0.067 in scope enter the set. The fifth place is a tie at 0.033 with no
finding on an anchor card; UXF-021 takes it: two sources agree on it, it is the oldest open UI
finding (R-05), and its fix is in the same component and the same test as UXF-034. T15, T1, T17,
T18 and T3 wait for R-09.

## Change set R-08

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-034 | `SupportMatrix/styles.module.css`: a bar's name column is as wide as its text (`max-content`), bars at least 17rem | `layout.overflow` on `reference/support` 4 → 0; ratchet red → green | `ux/tests/l2-findings.spec.ts::UXF-034 UXF-021 …` (2 tests), gate 1 |
| 2 | UXF-021 | descriptions wrap (10–13rem, no ellipsis); `custom.css`: on a page holding the matrix the table of contents column gives its width to the article (decision R-08, default yes) | support scroll containers at the gating viewports 8 → 0; descriptions cut 67 → 0 | same tests |
| 3 | UXF-035 | `custom.css`: `.markdown .b10x-chip` at 0.875rem | pipeline chips 10.4 → 14 px | `ux/tests/l2-findings.spec.ts::UXF-035 …` |
| 4 | UXF-005 | `website/src/theme/SearchPage/index.tsx` wraps the plugin's page and drops the query box's initial focus; the waiver is removed | `page-search` `keyboard.misses` 8 → 0; gate 5 waived 4 → 0 | `ux/tests/l2-layout.spec.ts::L2 page-search …` (keyboard skip-link-first, no waiver) |
| 5 | UXF-036 | `ux/core/round.ts`: the report and `round.json` name expected and missing units, the Playwright totals and the ratchet's verdict | report lines on completeness 0 → 3 | `ux/tests/l1-core.spec.ts::round names the units it expected …` (2 tests) |

One design decision, `decision.md` (UXF-021).

## Build and gate

Each asset first ran red on the old build, then green after its change:

- The two L1 tests failed with "not a function".
- The UXF-034/021 tests failed: 67 descriptions were cut, the matrix was 960 px wide in a 701 px column, and the bars ran past their box.
- The UXF-035 test failed at 10.4 px.
- The keyboard walk of `page-search` failed with 8 misses.

The gate ran on this tree, using `task ux-check`'s steps with 2 workers. Host load was 40 to 140 on 20 CPUs.

- `tsc` exited 0 (24 s), and `findings` exited 0.
- Playwright: the first run had 144 passed and 7 failed in 18.0 min. All 7 failures were timeouts at 60 s, on `reference/support` (4) and `status` (3). Under that load, the axe scan of the support page (13k nodes) took 35 to 98 s. A probe in the same conditions with the matrix's old layout took just as long.
- Rerun of the failed units: all passed. Support took 22 to 42 s.
- `ratchet`: `--update` took `page-search keyboard.misses` 8 → 0, then the ratchet exited 0.
