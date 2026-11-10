# R-05 review: report reviewer

**Verdict: not fit to send as is.** The numbers are right. But the report shows a ratchet regression
without marking it: `task-cir1-interfaces-generics` takes 6 actions, 10 looks and 7 steps against a
baseline of 2/3/3, while gate 4 reads "6 pass". The measured tree would fail `task ux-check`. As in
R-03, the report calls a dirty tree "commit `34edbf7`". It also says nothing about the operator's
goal, and the site has 11.7 % more words than at R-03. Default action: send the report with the § 4
caveats. Change the commit line to "`34edbf7` + regenerated `reference/cli.md` + untracked page
scenario", and mark the cir1 row as a ratchet regression caused by content growth, not by navigation.

Inputs: `report.md`, `round.json`, `shots/` (4 contact sheets viewed: support, cir-1 reference,
use-cases, read-a-quarkus-project), `notes.md`, `R-03/report.md`, `R-03/round.json`,
`R-03/review-report.md`, `R-04/triage.md`, `trend.md`, `ux/core`, `ux/tests`, `ux/cli.ts`,
`ux/adapter.ts`, `ux/baseline.json`, `Taskfile.yml`. The raw files are `ux/.out/round/results/*.json`
(282 = 276 page units + 6 tasks) and `ux/.out/round/playwright.json`. There were no recordings and no
questionnaire. I recomputed the numbers with my own script, not with `core/results.ts`. Two claims
were checked with Playwright on `website/build`, served on 127.0.0.1. Nothing was rerun.

## § 1 Are the numbers right and meaningful?

**Recomputed from raw: every headline number matches the report.**

| number | report | recomputed | note |
|---|---|---|---|
| gate1 / gate2 checked, pass | 92 / 92 | 92 / 92 | 23 pages × 2 viewports × 2 themes |
| gate5 pass, waived | 88, 4 | 88, 4 | all 4 are `page-search` skip-link-first, UXF-005 |
| gate4 reached | 6/6 | 6/6 | reach only; see the cir1 row |
| keyboard.misses | 8 | 8 | **4 defects counted twice** (T1) |
| axe crit/ser/mod/min | 0/0/0/0 | 0/0/0/0 | no axe rule fires at any viewport |
| layout_shift after_ready / load max | 0 / 0 | 0 / 0 | no `-1` sentinel in any file |
| scroll containers, gating | 18 | 18 | support 8, static-analysis 4, netcheck 4, crates 2; all `allowed: false` (T3) |
| screen-only actions / looks / steps | 19 / 27 / 25 | 19 / 27 / 25 | trend row agrees |
| measured-only rows (4 viewports) | as printed | identical | |
| playwright | n/a | 306 expected, 0 skipped, 0 failed, 0 flaky | |

**Against the baseline** (my own reduction, same rules as `core/ratchet.ts` `compare`):

- `task-cir1-interfaces-generics`: actions 2 → 6, looks 3 → 10, steps 3 → 7. All three exceed their
  tolerance. The report prints the baseline beside them with no mark (T8).
- `page-docs-guides-read-a-quarkus-project` has no baseline entry. Its scenario is untracked. The
  ratchet would report 9 "measured but not in the baseline" values.
- Every other value equals its baseline.

**R-05 against R-03** (R-04 has no measure; the report's "last round" column is R-03, unnamed, T9):

- Pages: 21 → 23 (`docs/use-cases`, `docs/guides/read-a-quarkus-project`).
- Screen-only: +4 actions and +7 looks, all on cir1. `task-install` looks 2 → 1 was already taken
  into the baseline in R-03.
- Gating scroll containers: 14 → 18. This cannot be traced by page, because R-03's raw files are
  gone (T4).
- Measured-only scroll containers: 1440x900 0 → 4, 1024x768 42 → 50, 768x1024 6 → 8, 390x844 110 →
  122. See § 2.
- Measured-only axe serious+critical at 768x1024 and 390x844: 42 → 0. The hamburger `target-size`
  defect R-03 § 2 named is fixed.
- Clipped text at 1024x768 is now only the support matrix (222). The navbar title truncation R-03 § 2
  named is gone. The home page no longer shows 0.1.2. Every current-release mention says 0.6.1; `vision.md` names
  0.1.0 and 0.2.0 only as milestone history.

**Do the metrics mean what their names say?**

- The cir1 regression measures 14 px of table of contents, not navigation.
  - Verified with Playwright at 1366x768: on `/codespec/docs/reference/cir-1` the TOC entry
    "Interface" sits at y 759–782 in a 768 px viewport.
  - The generated reference gained `Bean`, `Entry` and `Inject` (Java reader, Quarkus work). They
    sit above "Interface" in the TOC and push it off the first screen.
  - In R-03 the bot clicked that entry. Now the navbar "Reference" lands on the right page, but the
    bot follows "The cir/1 model" away and back, then searches (T11). The site's UI did not change
    here; the generated content grew.
- `keyboard.misses` 8 is 4 failing units (T1).
- "always-visible misses 92" at 768x1024 and 390x844 is all `theme toggle:missing`. That is the
  collapsed navbar, by design (T5).
- "scroll containers" counts wide tables. All 18 gating ones are flagged not allowed (T3).
- The header "Commit `34edbf7`" names a dirty tree. `website/docs/reference/cli.md` is modified and
  `ux/scenarios/page-docs-guides-read-a-quarkus-project/` is untracked (T2). The build
  (`website/build`, 09:56:45) is newer than the `cli.md` edit (09:56:22), so the regenerated page was
  measured. The date is the commit date; this time it is also the run date.
- `reference/crates.md` is stale on this branch (notes). Its 2 gating scroll containers come from a
  stale page.

**Can the metrics see the operator's goal?** No, as in R-03: no metric counts words, page length or
screens. The cir1 regression is the only measured number the goal touched, and it moved the wrong way
because a generated page grew.

**Content since R-03** (`wc -w` on `website/docs/**/*.md{,x}`):

| tree | all pages | hand-written | generated (`examples/*`, `reference/{cli,cir-1,crates}`) |
|---|---|---|---|
| R-03 (`dc18350`) | 28,735 | 11,644 | 17,091 |
| R-04 (`44e2a15`) | 28,436 | 11,346 | 17,090 |
| R-05 (measured tree) | 32,089 | 13,143 | 18,946 |

- R-04 cut 299 words.
- Feature work since then added 3,653:
  - new pages `use-cases` (+891) and `read-a-quarkus-project` (+637);
  - `getting-started` +137, `concepts/pipeline` +56;
  - generated `reference/cli` +1,153, `reference/cir-1` +469, `reference/crates` +242.
- Net since R-03: **+3,354 words (+11.7 %)**.

## § 2 Findings the report misses

- **The tree fails the every-merge gate.** `node cli.ts ratchet` on these values reports 3 "worse"
  (cir1) and 9 "missing value" (the untracked Quarkus scenario). The report shows gate 4 as 6/6 pass
  and lists no failure kind.
- **The cir1 task regressed because a generated page grew** (§ 1). Possible fixes, not chosen here:
  a shorter TOC on `reference/cir-1`, a hub link to "Interface" above the fold, or a ratchet decision.
- **The content goal moved backwards:** +11.7 % words since R-03, and 2 more pages. The feature work since
  R-04 added twelve times the 299 words R-04 cut. The report says nothing about content.
- **The support matrix now scrolls sideways at every desktop width, 1440x900 included.** The
  matrix is 848 px in 821 px (27 px), and the legend 829 in 821 (8 px)
  (`page-docs-reference-support__1440x900__*.json`). This started with the C column
  (`website/data/support-matrix.json`, `reader-c`, `writer-c`). In R-03 it was 0 at 1440x900. Not
  gated and not named.
- **Generated reference tables grow the scroll count:** `reference/cli` 26 at 1024x768 (R-03: 20,
  from the new `query`, `eval` and `check` commands) and `reference/cir-1` 56 at 390x844 (R-03: 46).
- **111 support-matrix descriptions still end in an ellipsis** on every unit of
  `/docs/reference/support`: 222 per desktop viewport over 2 themes, 286 at 390x844. Named in R-03
  § 2; no finding was filed.
- **The cir/1 reference intro is out of date:** "One source module: a Go package, a Rust crate
  module, a Java compilation unit set" (`shots/sheet-page-docs-reference-cir-1.jpg`), while the
  support matrix lists a C reader. The intro is generated from the model's doc comment.

## § 3 Tool defects

T1 to T7 from `R-03/review-report.md` § 3 all still hold. `ux/core/results.ts`, `round.ts`,
`ux/adapter.ts` and `ux/tests/global-setup.ts` are unchanged since R-03.

- **T1** holds. `ux/core/results.ts` `toMeasured`: all 4 `page-search` units have
  `skipLinkFirst: false, skipMovesFocus: false`, giving 8 misses for 4 failures.
- **T2** holds. `ux/core/round.ts` `writeRound`: `34edbf7` with a modified `reference/cli.md` and an
  untracked scenario. The trend row carries the same bare sha.
- **T3** holds. `ux/adapter.ts` `scrollContainersAllowed`: 18/18 gating containers are
  `allowed: false` (`div.b10x-table-wrap`, the hashed SupportMatrix divs). `clipped` (444 gating) is
  still collected and never reported.
- **T4** holds. `ux/tests/global-setup.ts` deletes the previous results. Input: the 14 → 18 gating
  scroll containers cannot be traced by page.
- **T5** holds. `ux/adapter.ts` `alwaysVisible`: 92 + 92 `theme toggle:missing` at 768x1024 and
  390x844.
- **T6** holds. `ux/core/round.ts` `missing_inputs` are fixed strings. They are true this round.
- **T7** holds, latent. `ux/core/round.ts` `Math.max(0, …)`. No `-1` in R-05.
- **T8** (new) `ux/core/round.ts` `writeRound`: the round never runs `ratchet.ts` `compare`. A worse
  value is printed beside its baseline with no mark. A scenario with no baseline is not flagged. Gate
  4 counts only reach (`ux/tests/l3-screen-only.spec.ts`). Input: `task-cir1-interfaces-generics.json`
  6/10/7 against baseline 2/3/3 is reported as a gate 4 pass; the Quarkus scenario has no baseline.
- **T9** (new, minor) `ux/core/round.ts` `prevId`: "last round" is the latest round that has a
  `round.json`, here R-03, and the report does not name it. Input: R-05's "last round" column, with
  R-04 present and holding only `triage.md`.
- **T10** (new) `ux/core/screen-only.ts` `visibleControls`: the role is taken from the tag before an
  explicit `role`. So `<a role="button">`, a Docusaurus sidebar category, becomes a "link", and
  `getByRole('link')` cannot find it. Input: the cir1 trace step `skip link "Reference": not found by
  role and name`. Checked with Playwright: on `/codespec/docs/reference/cir-1`, the sidebar
  "Reference" is `a[role=button][aria-expanded]`. The bot can never open a collapsed sidebar
  category.
- **T11** (new, design limit) `ux/core/screen-only.ts` `runScreenOnly`: on the truth path with the
  target just below the fold, a strong link in view ranks above one PageDown. The bot leaves the right
  page. Input: the cir1 trace, steps 1–3 (`reference/cir-1` → `concepts/cir-1` → `reference/cir-1`).
  A 14 px content shift costs 4 actions and 7 looks.

## § 4 Caveats to send with the round

1. The measured tree is `34edbf7` plus a regenerated `reference/cli.md` and an untracked page
   scenario. It is not a clean commit.
2. The tree fails the ratchet: cir1 2/3/3 → 6/10/7, and the Quarkus page has no baseline. Gate 4
   "6 pass" checks reach only.
3. The cir1 regression comes from three new definitions in the generated cir/1 reference. They push
   the TOC entry "Interface" 14 px below the fold. The UI did not change.
4. No metric measures structure or the amount of content. By `wc -w` the site grew 11.7 % since R-03
   (+3,354 words, 2 new pages). R-04's cut was 299 words.
5. `keyboard.misses` 8 is 4 failing units on `/codespec/search`, waived by UXF-005.
6. The mobile rows' "always-visible 92" are the collapsed navbar, by design. Their axe count is now 0:
   the hamburger target size is fixed.
7. The support matrix scrolls sideways at 1440x900 since the C column was added. It is measured, not
   gated.

---

**Verdict:** not fit to send as is. Send it with § 4, the corrected commit line and the cir1 row
marked as a ratchet regression.

**Numbers that differ:** none of the headline numbers. Against the baseline: cir1 actions 2 → 6,
looks 3 → 10, steps 3 → 7; one scenario has no baseline. Against R-03: gating scroll containers
14 → 18; measured-only scroll containers 0/42/6/110 → 4/50/8/122; mobile axe 42 → 0; words
28,735 → 32,089.

**Tool defects:**
- T1–T7 still hold (the files are unchanged).
- T8 `round.ts` runs no ratchet: a worse value and a missing baseline go unmarked.
- T9 `round.ts` "last round" is R-03 and is not named.
- T10 `screen-only.ts` treats `<a role="button">` as a link, so sidebar categories are skipped.
- T11 `screen-only.ts` prefers a strong link over one PageDown on the truth page.
