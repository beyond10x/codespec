# R-07 review: report reviewer

**Verdict: fit to send.** Every headline number matches the raw files. The values pass the ratchet:
231 baseline values, none worse, none better, none missing. The new "Regression assets" section
matches `playwright.json` (38 = 20 L1 + 18 `l2-findings`, all passed). R-06's five targets all
moved or held. One thing the report shows without saying it: "scroll containers 18 → 24" is a side
effect of UXF-024. The larger table code makes three more tables scroll sideways at the gating
viewports, and the report names no page. Default action: send the report with the § 4 caveats.

Inputs: `report.md`, `round.json`, `shots/` (30 files), `notes.md`, `R-06/report.md`,
`R-06/round.json`, `R-06/triage.md`, `R-06/review-report.md`, `trend.md`, `ux/cli.ts`, `ux/core`,
`ux/tests`, `ux/adapter.ts`, `ux/playwright.config.ts`, `ux/baseline.json`. The raw files are
`ux/.out/round/results/*.json` (282 = 276 page units + 6 tasks) and `ux/.out/round/playwright.json`.
There were no recordings and no questionnaire. I recomputed with my own script, not with
`core/results.ts`. Checks run:

- two short Playwright runs on `website/build`, served on 127.0.0.1;
- the R-05 and R-06 Rust assets, from the existing test binary of `crates/docs`.

That binary was built at 12:21, after the round, from a tree that another session is editing.
`crates/docs/src/main.rs` and `status.rs` hold two new red tests (UXF-019 and the home lede's C
writer). `website/` is identical to `90e3ec1`, and the R-05 and R-06 tests read only `website/`.
Nothing was rerun.

## § 1 Are the numbers right and meaningful?

**Recomputed from raw: every headline number matches the report.**

| number | report | recomputed | note |
|---|---|---|---|
| gate1 / gate2 checked, pass | 92 / 92 | 92 / 92 | 23 pages × 2 viewports × 2 themes |
| gate5 pass, waived | 88, 4 | 88, 4 | all 4 are `page-search` skip-link-first, UXF-005 |
| gate4 reached | 6/6 | 6/6 | |
| keyboard.misses | 8 | 8 | 4 failing units counted twice (T1) |
| axe crit/ser/mod/min | 0/0/0/0 | 0/0/0/0 | no axe rule fires at any viewport |
| layout_shift after_ready / load max | 0 / 0 | 0 / 0 | 0 in all 276 units |
| scroll containers, gating | 24 | 24 | support 8, static-analysis 4, netcheck 4, crates 4, pipeline 2, cli 2 |
| screen-only actions / looks / steps | 15 / 20 / 21 | 15 / 20 / 21 | trend row 15 / 20 agrees |
| measured-only rows (4 viewports) | as printed | identical | scroll containers 8 / 48 / 12 / 130 |
| regression assets | 38 passed, 0 failed | 38 expected | `l1-core` 20, `l2-findings` 18 (the 18 rows of the table) |
| playwright | n/a | 344 expected, 0 skipped, 0 failed, 0 flaky | plus l2-layout 277, round-shots 23, l3 6; 2 workers, 0 retries |

**Against the baseline** (my own reduction, same rules as `core/ratchet.ts` `compare`): 29 scenarios,
231 values, every value equal to its baseline.

**R-07 against R-06:** every per-scenario value and every task path is unchanged. Only the
scroll containers moved. Gating went 18 → 24. Measured-only went 4 → 8 (1440x900), 50 → 48
(1024x768), 8 → 12 (768x1024) and 122 → 130 (390x844).

**Did R-06's change set move its targets?** Now the round shows #1 and #2. I checked #3 and #4.

| # | finding | target | moved |
|---|---|---|---|
| 1 | UXF-024 | `td code` 12.67 → 14 px, `th` 10.88 → 14 px | yes on every Markdown table. Playwright at 1366x768: 1,664 `td code` on 17 pages, all 14 px. 313 `th` outside Status and the support matrix, all 14 px and none uppercase. Not on Status (§ 2). Side effect: 6 more gating scroll containers (§ 2) |
| 2 | UXF-026 | asset results in the round 0 → 38 | yes. `round.json` `regression_assets` and the report section agree with `playwright.json`. The Rust assets are still outside the round (T12) |
| 3 | UXF-022 | diagram 2 → 1 pages, orders run 2 → 1, counts 1 → 0, "not a language model" 2 → 1 | yes. `each_claim_is_on_one_docs_page` passes. Grep: the claim is in `index.md:34` only. `use-cases.md:16` mentions "a language model" in another sense. The run and the counts are in `getting-started.md` only |
| 4 | UXF-023 | pages stating the run twice 4 → 0 | yes. `the_run_is_stated_once_in_the_header_card` passes. "installed codespec" is only in the `source:` line of the four pages |
| 5 | UXF-018 (decision no) | cir1 task stays 2/3/3 | yes, 2/3/3, "Reference" → "Interface" |

R-05's assets still pass: `each_start_here_claim_is_on_one_page`, `the_overview_names_pages_as_the_sidebar_does`, `every_page_has_one_name` and `every_language_list_names_every_adapter`. The last one fails in that binary only because of the other session's uncommitted, wider version of the test.

**Do the metrics mean what their names say?**

- The rotating card is `task-reach-reports`. Its value 3/4/4 is comparable with R-06: same card,
  same start page, same path.
- "scroll containers 24" counts table wrappers. The 6 new ones are real sideways scrolling, one
  per theme:
  - `concepts/pipeline` "cir/1 | Rust | Decided by", 754 px in 703 at 1280x720;
  - `reference/cli` "Argument … Meaning", 735 in 703 at 1280x720;
  - `reference/crates`, 776 in 768 at 1366x768.

  Playwright with R-05's sizes forced back (code 12.672 px, headers 10.88 px uppercase) brings the
  pipeline and cli tables to 0 overflow and crates to 727 px, under 768. So UXF-024 caused all
  three. The metric is measured, not gated, so the ratchet passes (T3, T14).
- `keyboard.misses` 8 is 4 failing units (T1).
- "always-visible misses 92" at 768x1024 and 390x844 is all `theme toggle:missing`, the collapsed
  navbar (T5).
- Commit and date are right for the site. The Playwright run started at 12:03:11, after the commit
  at 12:02:28. The build (11:56) has no `website/` source newer than itself. The tree was dirty in
  `ux/` only, and the findings table reads that dirty state (T2).
- `reference/crates.md` is stale on this branch (notes). Its 4 gating scroll containers come from a
  stale page.

**Can the metrics see the operator's goal?** No, as in R-03, R-05 and R-06. By `wc -w` on
`website/docs/**/*.md{,x}`:

| tree | all pages | hand-written | generated (`examples/*`, `reference/{cli,cir-1,crates}`) |
|---|---|---|---|
| R-03 (`dc18350`) | 28,735 | 11,644 | 17,091 |
| R-06 (`647f0d8`) | 32,035 | 13,087 | 18,948 |
| R-07 (`90e3ec1`) | 31,952 | 13,004 | 18,948 |

- R-06's change set cut 83 words (−0.3 %), all hand-written, as the triage says.
- Since R-03: +3,217 words (+11.2 %). Over the three rounds R-05 to R-07, the pages lost 137 words.

## § 2 Findings the report misses

- **UXF-024 made three more tables scroll sideways at a gating viewport.**
  - At 1280x720, `concepts/pipeline` (51 px) and `reference/cli` (32 px) now scroll.
  - At 1366x768, `reference/crates` now scrolls (8 px).
  - At 1440x900, `concepts/static-analysis` and `examples/netcheck` now scroll too (R-06: support
    only).
  - Tables that already scrolled at 1280x720 are wider: static-analysis 798 → 890, netcheck
    820 → 903, crates 727 → 776.

  The report prints 24 against 18 without a page or a cause.
- **UXF-024 missed the Status table.** Its 9 row headers (Model, Readers, Passes, Writers, Harness,
  Examples, Interfaces, Conformance, Analysis) are 10.88 px, uppercase, at 1366x768. The asset reads
  only the first `td code` and the first `th` of `/codespec/docs/reference/cir-1`, so it passes. The
  support matrix keeps its own sizes by choice (172 of 175 headers below 14 px, minimum 12 px;
  UXF-021).
- **The findings table reads uncommitted files.** UXF-018 is "closed" in `report.md`, but in
  `90e3ec1` it is open. The closure, the decision's answer and the checklist exception are
  uncommitted (notes say so; the report does not) (T2).
- **The content goal did not move.** −83 words this round. The report says nothing about content.
- **Carried, unchanged:**
  - UXF-025 is open and the home page still says "writes Go or Rust from it" (y 430) above "Go, Rust
    and C writers" (y 731) at 1366x768;
  - the support matrix scrolls sideways at every desktop width (848 px in 766 at 1366x768; UXF-021);
  - 111 descriptions are cut on every unit of `/docs/reference/support` (222 per viewport, 286 at
    390x844; `clipped`, never reported, T3);
  - search paths read "Reference › Reference" in 2 of 6 task traces (reach, trace). This is now a
    decided exception (decision R-06, no).

## § 3 Tool defects

Since R-06 (`647f0d8`), `ux/core/round.ts`, `ux/core/specs.ts` (new), `ux/playwright.config.ts`,
`l1-core.spec.ts` and `l2-findings.spec.ts` changed. `results.ts`, `layout.ts`, `ratchet.ts`,
`screen-only.ts`, `adapter.ts`, `cli.ts` and `global-setup.ts` did not.

- **T1** holds. `ux/core/results.ts` `toMeasured`: the 4 `page-search` gating units have
  `skipLinkFirst: false, skipMovesFocus: false`, giving 8 misses for 4 failures.
- **T2** holds and fires, mildly. `ux/core/round.ts` `writeRound` prints `rev-parse --short HEAD` with
  no dirty check. The site equals `90e3ec1`. `ux/findings/UXF-018.md`, `ux/checklist.md` and
  `R-06/decision.md` were modified, and the findings table shows UXF-018 closed.
- **T3** holds. `ux/adapter.ts` `scrollContainersAllowed` names `table` and `.table-wrapper`. The
  site wraps tables in `div.b10x-table-wrap`, so all 222 containers at all viewports (24 gating) are
  `allowed: false`. `clipped` (222–286 per viewport, all on the support page) is collected and never
  reported.
- **T4** holds. `ux/tests/global-setup.ts` deletes the previous results. To find which pages added
  the 6 scroll containers, I had to use R-06's review text and a forced-style Playwright run.
- **T5** holds. `ux/adapter.ts` `alwaysVisible`: 92 + 92 `theme toggle:missing` at 768x1024 and
  390x844.
- **T6** holds. `ux/core/round.ts` `missing_inputs` are fixed strings. They are true this round.
- **T7** holds, latent. `ux/core/round.ts` `Math.max(0, …)`. No `-1` in R-07.
- **T8** holds, latent. `ux/core/round.ts` never runs `ratchet.ts` `compare`. This round nothing is
  worse or missing.
- **T9** holds, minor. `ux/core/round.ts` `prevId`: the "last round" column does not name its round.
  This round it is R-06, which is right.
- **T10** holds, latent. `ux/core/screen-only.ts` `visibleControls` takes the role from the tag
  before `role`. No task hit it.
- **T11** holds, latent. `ux/core/screen-only.ts` `runScreenOnly` prefers a strong link in view over
  one PageDown. It did not fire.
- **T12** partly fixed. `ux/core/specs.ts` `specPattern('round')` now runs L1 and `l2-findings`
  (`playwright.json`: `l1-core` 20, `l2-findings` 18). The Rust assets in `crates/docs` still do not
  run. Two of R-06's four built targets (#3 UXF-022, #4 UXF-023) have no result in `report.md`. The
  report says this.
- **T13** (new, latent) `ux/core/specs.ts` `assetResults` and `ux/core/round.ts` `writeRound`:
  - With no `playwright.json`, or none of its asset files, the section says "0 passed, 0 failed" and
    shows an empty table, with no warning.
  - A `skipped` or `flaky` test counts as failed.
  - A spec with no test entries counts as passed.

  Input: `assetResults` on `{suites:[{file:'l2-findings.spec.ts', specs:[{tests:[{status:'skipped'}]}, {tests:[{status:'flaky'}]}, {tests:[]}]}]}` gives ok false, false, true. All 38 were `expected` this round.
- **T14** (new) `ux/core/round.ts` prints "scroll containers" as one total, with baseline "–" and no
  page list. A change set's side effect shows only as 18 → 24. Input: `report.md` line 40 against
  the raw results (the new pipeline, cli and crates containers).

## § 4 Caveats to send with the round

1. The measured site is commit `90e3ec1`, R-06's change set. The ratchet passes: 231 values, none
   worse. The tree was dirty in `ux/` only, and the findings table shows UXF-018 closed from that
   uncommitted state.
2. All five R-06 targets moved or held. The round shows #1 (UXF-024 asset) and #2 (38 assets). #3
   and #4 were checked with the Rust assets outside the round (T12).
3. UXF-024 widened tables: "scroll containers 18 → 24" is `concepts/pipeline` and `reference/cli`
   now scrolling sideways at 1280x720, and `reference/crates` at 1366x768. Measured, not gated.
4. The Status table's 9 row headers are still 10.88 px uppercase. The UXF-024 asset does not look
   there.
5. No metric measures structure or the amount of content. By `wc -w` the pages lost 83 words
   (−0.3 %). The site is still +11.2 % over R-03.
6. `keyboard.misses` 8 is 4 failing units on `/codespec/search`, waived by UXF-005.
7. The mobile rows' "always-visible 92" are the collapsed navbar, by design.

---

**Verdict:** fit to send, with § 4.

**Numbers that differ:** none of the headline numbers, and none against the baseline. Against R-06:
gating scroll containers 18 → 24, measured-only 4/50/8/122 → 8/48/12/130. Words 32,035 → 31,952.

**Tool defects:**
- T1, T3, T4, T5, T6 hold and show.
- T2 holds and fires mildly (uncommitted `ux/` state under a bare sha).
- T7, T8, T10, T11 hold in code but did not fire. T9 holds, minor.
- T12 is partly fixed: Playwright assets run, Rust assets do not.
- T13 (new, latent): `specs.ts` `assetResults` says "0 passed, 0 failed" with no `playwright.json`,
  counts skipped or flaky as failed, and counts an empty spec as passed.
- T14 (new): `round.ts` prints scroll containers as a bare total with no pages.
