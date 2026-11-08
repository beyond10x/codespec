# R-03 review: report reviewer

**Verdict: not fit to send as is.** The numbers are right, but the report calls a dirty tree "commit
`1988d91`", and it reads as a check of the operator's goal (structure, less content) when no metric in
the harness can see that goal. Default action: send it with the § 4 caveats and the commit line
changed to "`1988d91` + uncommitted prose pass over 11 pages". Do not cite R-03 as evidence for or
against the content goal.

Inputs: `report.md`, `round.json`, `shots/` (2 contact sheets viewed: home, docs), `notes.md`,
`R-02/report.md` and `round.json`, `trend.md`, `ux/core`, `ux/tests`, `ux/cli.ts`, `ux/adapter.ts`,
`ux/baseline.json`. Raw files are `ux/.out/round/results/*.json` (258 = 252 page units + 6 tasks) and
`ux/.out/check/results` (90). There were no recordings and no questionnaire. I recomputed the numbers
with my own script, not `core/results.ts`. Nothing was rerun.

## § 1 Are the numbers right and meaningful?

**Recomputed from raw: every headline number matches the report.**

| number | report | recomputed | note |
|---|---|---|---|
| gate1 / gate2 checked, pass | 84 / 84 | 84 / 84 | |
| gate5 pass, waived | 80, 4 | 80, 4 | all 4 are `page-search` skip-link-first, UXF-005 |
| gate4 reached | 6/6 | 6/6 | |
| keyboard.misses | 8 | 8 | **4 defects counted twice** (T1) |
| axe crit/ser/mod/min | 0/0/0/0 | 0/0/0/0 | |
| layout_shift after_ready / load max | 0 / 0 | 0 / 0 | no `-1` sentinel in any file |
| scroll containers, gating | 14 | 14 | all 14 marked `allowed: false` (T3) |
| screen-only actions / looks / steps | 15 / 21 / 21 | 15 / 21 / 21 | |
| measured-only rows (4 viewports) | as printed | identical | |
| `.out/check` totals | n/a | identical to round | |
| playwright | n/a | 280 expected, 0 skipped, 0 failed | |

**R-03 against R-02:** `round.json` is identical except for the 390x844 scroll containers, which go
from 108 to 110. That +2 cannot be traced to a page because R-02's raw files were overwritten (T4).
The prose pass changed nothing else that the harness measures.

**Do the metrics mean what their names say?**

- `keyboard.misses` 8 means 4 failing units. A unit where the skip link is not first also counts as
  "skip link does not move focus", which was never tested (T1).
- At 768x1024 and 390x844, "always-visible misses 84" is entirely `theme toggle:missing`. Docusaurus
  moves the toggle into the hamburger menu below 997 px. This is the site's design, not a miss. The
  adapter gives the toggle no `minWidth` (T5).
- "scroll containers" counts wide tables. Every one is flagged as not allowed because the adapter's
  allowlist names `.table-wrapper` but the site uses `div.b10x-table-wrap` (T3).
- The report header says "Commit `1988d91`", but the measured tree had 11 modified pages. The build
  (`website/build`, 00:27:12) is newer than the last edit (00:26:10), so the pass was measured. The
  label and the trend row say otherwise (T2).

**Can the metrics see the operator's goal?** No.

- Structure:
  - The screen-only user stops at the target URL or anchor and never reads the page.
  - 4 of 6 tasks go Search → result option, which skips the sidebar, hub pages and link names. Only
    `task-install` ("Get started") and `task-cir1-interfaces-generics` ("cir/1" → "Interface") walk
    the navigation.
  - No trace contains a PageDown.
- Amount of content:
  - No metric counts words, page length or screens.
  - Contact sheets show only the first viewport.
- The one measured number that would move if content were cut is **scroll containers** (14 gating,
  42 / 6 / 110 measured-only). It counts wide tables, mostly generated: the `ArgumentRequiredDefault`
  tables on `reference/cli` (20 at 1024x768) and the `FieldTypeRequired` tables on
  `reference/cir-1` (46 at 390x844). It is reported, not gated.
- `screen_only.looks` / `actions` would move only if a target link left or entered the first screen.
- The `clipped` count would also move, but it is collected and never reported (T3).

**Content actually removed** (`wc -w` on the markdown source, HEAD vs worktree, 11 edited pages):
11,363 → 11,278 words, **−85 (−0.7 %)**. Six pages grew: pipeline +24, vision +21,
translate-go-to-rust +17, add-an-adapter +16, concepts/cir-1 +7, getting-started +1. Three shrank:
map-a-repository −79, index −71, static-analysis −21. The site total after the pass is 28,734,
matching the notes' "about 28,700". Of that, about 15,600 words are on generated pages the pass did
not touch.

**Proposed cheap metrics (two):**

1. `content.screens`: per page, `main article` scrollHeight ÷ 768 at 1366x768 plus its word count,
   summed lower-is-better, with generated pages in their own column. Taken in the L2 spec that
   already loads every page.
2. `screen_only.nosearch.*`: the 6 task cards rerun with the search box hidden, so actions and looks
   measure the sidebar, hub pages and link names instead of the search index.

## § 2 Findings the report misses

- **The home page shows version 0.1.2, but the workspace is 0.6.0** (`Cargo.toml:6`, commit
  "Release 0.6.0"). The hero eyebrow reads "Code intermediate representation · 0.1.2"
  (`website/product.json:6`, visible in `shots/sheet-page-home.jpg`). A second "0.1.2. Here is exactly
  what exists." is at `product.json:120`. The prose pass did not touch this.
- **The goal was not moved:** −0.7 % words, and 6 of the 11 edited pages grew (§ 1). The report says
  nothing about content.
- **The hamburger button is under 24 px:** axe `target-size` (serious) on `.navbar__toggle`, on all
  42 units at 768x1024 and all 42 at 390x844. This is the whole of the "axe serious+critical 42" in
  both rows. It is unnamed and has no finding.
- **The site name is truncated at 1024x768:** `b.navbar__title.text--truncate "codespec"` is clipped
  on all 21 pages in both themes (`page-*__1024x768__*.json`, `overflow.clipped`). This is
  measured-only and not visible in any shot.
- **111 support-matrix descriptions end in an ellipsis** on every unit of `/docs/reference/support`.
  That is 222 per viewport over 2 themes, 286 at 390x844. The cause is
  `SupportMatrix/styles.module.css:416-421` (`max-width: 17rem; overflow: hidden; text-overflow:
  ellipsis`). The descriptions are in the content, but a reader cannot read them in place.
- **The navbar repeats the sidebar:** "Get started" and "Status" are both top-nav links and
  "Start here" sidebar items (`shots/sheet-page-docs.jpg`). This is relevant to the structure goal.
  It is observed in shots only, with no metric behind it.
- **Generated pages dominate the word count** (series 5,757, cir-1 reference 4,339, cli 2,514). Most
  measured table scroll comes from them. Any real content cut has to go through
  `crates/docs/src/`, which the report does not point out.

## § 3 Tool defects

- **T1** `ux/core/results.ts` `toMeasured`: adds `skipMovesFocus ? 0 : 1` even when `skipLinkFirst`
  is false and focus was never tested. Input: `page-search__1366x768__light.json` gives 2 misses for
  1 failure. `keyboard.misses` 8 means 4.
- **T2** `ux/core/round.ts` `writeRound`: `commit` is `rev-parse --short HEAD` with no dirty check,
  and `date` is the commit date, not the run date. Input: R-03 shows `1988d91 (2026-11-08)` while 11
  pages were modified and the run was 2026-11-09 00:30.
- **T3** `ux/adapter.ts` `scrollContainersAllowed`: lists `.table-wrapper`, but the site uses
  `.b10x-table-wrap` and a hashed SupportMatrix class. All 14 gating containers are `allowed: false`.
  `allowed` and `overflow.clipped` (444 gating) are collected but never reported or used in
  `core/round.ts`.
- **T4** `ux/tests/global-setup.ts`: `rmSync(results)` deletes the previous round's raw files, and
  `round.json` keeps only totals for measured-only viewports. Input: the 390x844 change from 108 to
  110 scroll containers cannot be traced to a page.
- **T5** `ux/adapter.ts` `alwaysVisible`: `theme toggle` has no `minWidth: 997`. Input:
  `page-*__768x1024__*.json` and `__390x844__` give 84 + 84 "misses" that are the collapsed navbar.
- **T6** `ux/core/round.ts` `missing_inputs`: the L6, questionnaire and L4/L5 lines are fixed
  strings, not read from inputs. They are true this round, but a round with sessions would still print
  "none recorded".
- **T7** `ux/core/round.ts` `maxLoad` / `maxReady`: `Math.max(0, …)` hides the `-1` that
  `readShift` returns when the observer is unavailable. Latent: no unit had it in R-03.

## § 4 Caveats to send with the round

1. The measured tree is `1988d91` plus an uncommitted prose pass over 11 hand-written pages. It is not
   a clean commit.
2. Gates and metrics are unchanged from R-02, apart from +2 scroll containers at 390x844 whose
   source cannot be traced.
3. No metric measures structure or the amount of content. R-03 says nothing about the operator's goal.
4. The prose pass cut 85 words (0.7 %) from the 11 pages, and 6 of them grew.
5. `keyboard.misses` 8 is 4 failing units on `/codespec/search`, waived by UXF-005.
6. The mobile rows (768x1024, 390x844) "always-visible 84" are the collapsed navbar by design. Their
   "axe 42" is one real defect, an undersized hamburger, repeated on every page.

---

**Verdict:** not fit to send as is. Send with § 4 and the corrected commit line.

**Numbers that differ:** none of the headline numbers. `keyboard.misses` 8 is arithmetically right
but double-counts 4 defects. R-03 differs from R-02 only in 390x844 scroll containers, 108 → 110.

**Tool defects:**
- T1 `results.ts` counts untested skip-link focus as a second keyboard miss.
- T2 `round.ts` labels a dirty tree with the bare HEAD sha and the commit date.
- T3 `adapter.ts` allowlist misses `.b10x-table-wrap`; `allowed` and `clipped` are collected but never reported.
- T4 `global-setup.ts` deletes the previous raw results, so changes between rounds cannot be traced.
- T5 `adapter.ts` theme toggle has no `minWidth`, so the collapsed navbar counts as a miss.
- T6 `round.ts` missing-inputs lines are fixed strings.
- T7 `round.ts` `Math.max(0, …)` hides a `-1` layout-shift sentinel.
