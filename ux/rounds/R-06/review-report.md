# R-06 review: report reviewer

**Verdict: fit to send.** Every headline number matches the raw files. The tree is the clean commit
`647f0d8`, and the build has no source newer than itself. The values pass the ratchet: 231 baseline
values, none worse, none better, none missing. The report has two gaps. It cannot show whether four
of R-05's five changes held, because the round runs none of their regression assets. It also says
nothing about the content goal, which is flat. Default action: send the report with the § 4 caveats.

Inputs: `report.md`, `round.json`, `shots/` (31 files: 23 contact sheets, 4 fail shots; the home and
search sheets viewed), `notes.md`, `R-05/report.md`, `R-05/round.json`, `R-05/triage.md`,
`R-05/review-report.md`, `trend.md`, `ux/cli.ts`, `ux/core`, `ux/tests`, `ux/adapter.ts`,
`ux/playwright.config.ts`, `ux/baseline.json`, `Taskfile.yml`. The raw files are
`ux/.out/round/results/*.json` (282 = 276 page units + 6 tasks) and `ux/.out/round/playwright.json`.
There were no recordings and no questionnaire. I recomputed with my own script, not with
`core/results.ts`. Checks run:

- one Playwright run on `website/build`, served on 127.0.0.1 by `ux/core/site.ts`;
- the three Rust regression assets of R-05, from the existing test binary of `crates/docs`, which is
  newer than every file in `crates/docs/src`.

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
| scroll containers, gating | 18 | 18 | support 8, static-analysis 4, netcheck 4, crates 2; same pages as R-05 |
| screen-only actions / looks / steps | 15 / 20 / 21 | 15 / 20 / 21 | trend row 15 / 20 agrees |
| measured-only rows (4 viewports) | as printed | identical | identical to R-05 too |
| playwright | n/a | 306 expected, 0 skipped, 0 failed, 0 flaky | l2-layout 277, round-shots 23, l3 6 |

**Against the baseline** (my own reduction, same rules as `core/ratchet.ts` `compare`): 29 scenarios,
231 values, every value equal to its baseline. The Quarkus page now has its baseline entry.

**R-06 against R-05:** one change. `task-cir1-interfaces-generics` went 6/10/7 → 2/3/3, back to its
baseline. Every other per-scenario value, every measured-only row and the 18 gating scroll
containers are unchanged. The report's "last round" column is R-05 this time, still unnamed (T9).

**Did R-05's change set move its targets?** Only #4 is a round metric. I checked the rest myself.

| # | finding | target | moved |
|---|---|---|---|
| 1 | UXF-020 | sidebar group names 10.88 → 14 px | yes. Playwright at 1366x768: all six groups 14 px, `text-transform: none` |
| 2 | UXF-016 | places missing an adapter 10 → 0 | yes for the places the test reads (`every_language_list_names_every_adapter` passes). Three other lists still omit C (§ 2) |
| 3 | UXF-017 | use-cases 891 → 816 words, vision 755 → 723; repeated claims on Start here pages → 1 | yes. `wc -w`: 816 and 723; `each_start_here_claim_is_on_one_page` passes |
| 4 | UXF-015 | cir1 task 6/10/7 → 2/3/3; Interface and TypeParam links in the first screen | yes. Round: 2/3/3 via "Reference" → "Interface". Playwright: the index links sit at y 562–608 (1366x768) and 592–665 (1280x720) |
| 5 | UXF-018 (part) | old names on the overview 2 → 0; header card on every Start here page | yes. `the_overview_names_pages_as_the_sidebar_does` passes; `index.md` has its `lede`. Search paths still read "Reference › Reference" (all 3 search traces) |

**Do the metrics mean what their names say?**

- The cir1 row is comparable with R-05: same card, same start page. It recovered because the
  generated reference now lists its definitions first. The navbar and the bot are unchanged.
- The path still depends on the navbar's "Reference" opening the cir/1 reference directly. That is
  the half of UXF-018 that stays open. A reference index page would add a step to this task.
- `keyboard.misses` 8 is 4 failing units (T1).
- "always-visible misses 92" at 768x1024 and 390x844 is all `theme toggle:missing`, the collapsed
  navbar (T5).
- "scroll containers" counts wide tables; all 202 at every viewport are flagged not allowed (T3).
- Commit and date are right: the tree is clean apart from the round's own output. The build
  (11:21) has no source file newer than itself, and the run (from 11:30:37) started after the commit
  (11:30:15).
- `reference/crates.md` is stale on this branch (notes). Its 2 gating scroll containers come from a
  stale page.

**Can the metrics see the operator's goal?** No, as in R-03 and R-05. No metric counts words, page
length or screens. By `wc -w` on `website/docs/**/*.md{,x}`:

| tree | all pages | hand-written | generated (`examples/*`, `reference/{cli,cir-1,crates}`) |
|---|---|---|---|
| R-03 (`dc18350`) | 28,735 | 11,644 | 17,091 |
| R-05 (measured tree) | 32,089 | 13,143 | 18,946 |
| R-06 (`647f0d8`) | 32,035 | 13,087 | 18,948 |

- R-05's change set cut 54 words (−0.2 %): use-cases −75, vision −32, index +7, pipeline +44,
  generated +2.
- Status gained two entries in `website/data/status.json` (about 70 words), which `wc -w` on the
  pages does not see. The net is flat.
- Since R-03: +3,300 words (+11.5 %).

## § 2 Findings the report misses

- **The report cannot show whether R-05's changes held.** Round mode runs only `l2-layout`,
  `l3-screen-only` and `round-shots`. The L1 specs, `l2-findings.spec.ts` (UXF-015, UXF-020) and the
  Rust assets are not run (T12). Four of five targets have no number in `report.md`. § 1 has them,
  all held.
- **The home page still contradicts itself about C (UXF-016 closed, effect not gone).** At 1366x768
  the lede at y 430 says "writes Go or Rust from it". The meta line at y 731, on the same screen,
  says "Go, Rust and C writers" (`shots/sheet-page-home.jpg`). Source: `website/product.json` `lede`
  and `description` (lines 11–12). The regression asset reads only `product.meta`, the overview
  sentence and the program names, so it passes.
- **The cir/1 reference intro still names three languages:** "One source module: a Go package, a Rust
  crate module, a Java compilation unit set." (`website/docs/reference/cir-1.md:22`, generated from the
  model's doc comment). Named in R-05 § 2; no finding was filed. `concepts/pipeline.md`'s front-matter
  description says "the Go and Rust writers".
- **The content goal did not move.** −54 words in the pages and +2 Status entries, against +3,354
  since R-03. The report says nothing about content.
- **Carried from R-05, unchanged:**
  - the support matrix scrolls sideways at 1440x900 (848 px in 821, legend 829 in 821; UXF-021);
  - 111 descriptions end in an ellipsis on every unit of `/docs/reference/support` (222 per desktop
    viewport, 286 at 390x844; `clipped`, never reported, T3);
  - `reference/cli` makes 26 scroll containers at 1024x768 and `reference/cir-1` 56 at 390x844.
- **The navbar "Reference" still opens a single page.** All three search hits show "Reference ›
  Reference" (UXF-018, open, as the report lists).

## § 3 Tool defects

`ux/core/*`, `ux/adapter.ts`, `ux/cli.ts`, `ux/tests/global-setup.ts`, `l1-core`, `l2-layout` and
`l3-screen-only` are unchanged since R-03 (`dc18350`). Since R-05, only `l2-findings.spec.ts` gained
tests. Every defect still holds in code. Some do not show this round.

- **T1** holds. `ux/core/results.ts` `toMeasured`: the 4 `page-search` gating units have
  `skipLinkFirst: false, skipMovesFocus: false`, giving 8 misses for 4 failures.
- **T2** holds, latent. `ux/core/round.ts` `writeRound` prints the bare sha with no dirty check. This
  round the tree is clean, so `647f0d8` is right.
- **T3** holds. `ux/adapter.ts` `scrollContainersAllowed`: 202/202 containers (18 gating) are
  `allowed: false`. `clipped` (222–286 per viewport, all on the support page) is collected and never
  reported.
- **T4** holds. `ux/tests/global-setup.ts` deletes the previous results. Per-page comparison with
  R-05 was possible only from the R-05 review's text.
- **T5** holds. `ux/adapter.ts` `alwaysVisible`: 92 + 92 `theme toggle:missing` at 768x1024 and
  390x844.
- **T6** holds. `ux/core/round.ts` `missing_inputs` are fixed strings. They are true this round.
- **T7** holds, latent. `ux/core/round.ts` `Math.max(0, …)`. No `-1` in R-06.
- **T8** holds, latent. `ux/core/round.ts` never runs `ratchet.ts` `compare`. This round nothing is
  worse or missing, so nothing is hidden.
- **T9** holds, minor. `ux/core/round.ts` `prevId`: the "last round" column does not name its round.
  This round it is R-05, which is right.
- **T10** holds, latent. `ux/core/screen-only.ts` `visibleControls` takes the role from the tag
  before `role`. Playwright: all six sidebar groups are `A role=button aria-expanded`. No task hit it
  this round.
- **T11** holds, latent. `ux/core/screen-only.ts` `runScreenOnly` prefers a strong link in view over
  one PageDown. The cir1 target is now in the first screen, so it did not fire.
- **T12** (new) `ux/playwright.config.ts` `testMatch` in round mode, with `ux/cli.ts` `round`: the
  round runs no L1 spec, no `l2-findings.spec.ts` and no Rust asset. The report cannot say whether
  the last change set held. Input: `ux/.out/round/playwright.json` has only `l2-layout.spec.ts` (277),
  `round-shots.spec.ts` (23) and `l3-screen-only.spec.ts` (6). R-05's targets #1, #2, #3 and #5 have
  no number in `report.md`.

## § 4 Caveats to send with the round

1. The measured tree is the clean commit `647f0d8`, R-05's change set. The ratchet passes: 231
   values, none worse.
2. Only the cir1 task (6/10/7 → 2/3/3) shows a change set target in the report. The other four
   held by their own tests and by a Playwright check, not by the round (T12).
3. UXF-016 is closed, but the home page's first screen still says "writes Go or Rust" above "Go,
   Rust and C writers" (`website/product.json` `lede`). The cir/1 reference intro still names three
   languages.
4. No metric measures structure or the amount of content. By `wc -w`, the pages lost 54 words
   (−0.2 %) and Status gained two entries. The site is still +11.5 % over R-03.
5. `keyboard.misses` 8 is 4 failing units on `/codespec/search`, waived by UXF-005.
6. The mobile rows' "always-visible 92" are the collapsed navbar, by design.
7. The support matrix still scrolls sideways at 1440x900 and cuts 111 descriptions (UXF-021,
   measured, not gated).

---

**Verdict:** fit to send, with § 4.

**Numbers that differ:** none of the headline numbers, and none against the baseline. Against R-05:
cir1 actions 6 → 2, looks 10 → 3, steps 7 → 3 (screen-only totals 19/27/25 → 15/20/21). Words
32,089 → 32,035.

**Tool defects:**
- T1, T3, T4, T5, T6 hold and show.
- T2, T7, T8, T10, T11 hold in code but did not fire this round.
- T9 holds, minor: the column is right (R-05) but unnamed.
- T12 (new) `playwright.config.ts` round mode runs no regression asset, so the report cannot show
  whether the last change set held.
