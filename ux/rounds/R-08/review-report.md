# R-08 review: report reviewer

**Verdict: not fit to send as is.** The printed numbers match the raw files: every one of them is
right for the 285 page units and 6 tasks that exist. Three things make the report wrong or silent:

1. 3 of 288 page units have no result (all `/codespec/docs`, the docs landing page), and the report
   says "93 checked" with no word that 96 were expected. Their Playwright tests timed out at 60 s.
2. The ratchet is not run by the round, and it fails: `page-docs-reference-support layout.overflow`
   4 > baseline 0. The report prints "4" against "0" in a table and never says the ratchet is red.
3. A new hard-gate-1 failure on the support matrix (a 3-4 px overflow of the header "Evaluator
   write") shows at all 6 viewports. It is unwaived, has no finding in the report's table, and its "worst moment" shot
   shows the top of the page, where nothing is wrong.

Default action: before sending, (a) say in the report that 93 of 96 gating units were measured and
that the three `page-docs` units, rerun in isolation, pass (§ 1.2), or merge those result files
into `.out/round` and rewrite the report, (b) add one line to the report that the ratchet is red
on `layout.overflow` and why, (c) file the support overflow as a finding, (d) send with § 4.

Inputs: `report.md`, `round.json`, `shots/` (32 files), `notes.md`, `../R-07/report.md`,
`../R-07/round.json`, `../R-07/triage.md`, `../R-07/review-report.md`, `../trend.md`,
`target/round-R08.log`, `target/rewrite-round.ts`, `ux/baseline.json`, `ux/cli.ts`, `ux/core/*.ts`,
`ux/tests/*.ts`, `ux/adapter.ts`, `ux/playwright.config.ts`, `Taskfile.yml`. Raw files:
`ux/.out/round/results/*.json` (291 = 285 page units + 6 tasks) and `ux/.out/round/playwright.json`.
There were no recordings and no questionnaire. I recomputed with my own scripts
(`target/rr-recompute.mjs`, `target/rr-pw.mjs`), reading the result files directly and using only
`ratchet.ts` `compare` and `results.ts` `toMeasured` from the tools, to run the ratchet.
`core/round.ts` was not used to recompute anything.

## § 1 Are the numbers right and meaningful?

### 1.1 Recomputed from raw

| number | report | recomputed | note |
|---|---|---|---|
| gate1 checked / pass / fail | 93 / 89 / 4 | 93 / 89 / 4 | 4 = `reference/support` × 4 gating units, overflow of `span "Evaluator write"` (right 948 vs limit 944 at 1366x768; 784 vs 781 at 1280x720) |
| gate2 checked / pass | 93 / 93 | 93 / 93 | 0 misses at the gating viewports |
| gate5 checked / pass / waived | 93 / 89 / 4 | 93 / 89 / 4 | all 4 are `page-search` skip-link-first, waived by UXF-005 (open) |
| gate4 | 6 / 6 | 6 / 6 | all reached |
| layout.overflow | 4 | 4 | baseline 0 |
| keyboard.misses | 8 | 8 | 4 failing units counted twice (T1) |
| axe crit / ser / mod / min | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | no axe rule fires in any of the 285 units |
| layout_shift after_ready / load max | 0 / 0 | 0 / 0 | |
| scroll containers, gating | 8 | 8 | all on `reference/support`: legend 941 in 766 and matrix 960 in 766 at 1366x768 (941 / 960 in 701 at 1280x720), 2 per unit × 4 units |
| measured-only rows | 48 units each; overflow 2 / 2 / 2 / 2; always-visible 0 / 0 / 96 / 96; scroll containers 4 / 10 / 4 / 42 | identical | the overflow is `reference/support` again, 2 themes per viewport |
| screen-only actions / looks / steps | 15 / 20 / (per task) | 15 / 20 / 21 | every task has R-07's actions, looks and steps; `trace-options` is the rotating card of this round (index 7 mod 4 = 3, `reach-reports` was 6 mod 4 = 2) |
| trend row R-08 | 4, 0, 0, 4, 0, 4, 15, 20, 0 | same | gate columns count waived and unwaived failures |
| regression assets | 43 passed, 0 failed | 43 | `l1-core` 22 + `l2-findings` 21, all `expected`; the 21 table rows match |
| playwright (not in the report) | n/a | 362 tests, 355 expected, 7 unexpected | 3 timeouts (`page-docs`), 4 gate failures (`reference/support`); 2 workers, 0 retries, 26.1 min |

Numbers that differ from the report: none. Numbers that the report does not carry: see 1.2 and 1.3.

### 1.2 The three missing units (known fact 2, confirmed)

- Files missing from `ux/.out/round/results`: `page-docs` at 1366x768 light, 1366x768 dark and
  1280x720 dark. 24 pages × 12 = 288 page units expected; 285 exist. Gating units: 96 expected, 93
  exist. Of the 4 gating units of `/codespec/docs`, only 1280x720 light was measured.
- Cause in the log: the tests ran at 60 s (`Test timeout of 60000ms exceeded`) inside `keyboardWalk`
  (`layout.ts:196`, `:221`, `:230`: a `page.goto` and a `locator.evaluate` that were still
  running). They were tests 45 to 52, the first L2 units, running beside the heavy `l2-findings`
  tests (UXF-029 and UXF-030 took 28-37 s each) on a host the slot queue called busy. The five
  `page-docs` units that finished took 22 to 49 s; `reference/cli` took 5 s at the same viewports
  later. So the timeout is load, not a page defect. That is an inference; the rerun below tests it.
- A unit that times out never reaches `writeResult` (`l2-layout.spec.ts` writes at the end of the
  test), so it leaves no file and nothing in `report.md`, `round.json`, the gate counts, the
  trend row or the ratchet.
- The ratchet does not see it either. `page-docs` has 1 of 4 gating units measured, every value 0,
  and `compare` finds no problem. I ran `compare` on the baseline (30 scenarios) and the measured
  values (30 scenarios): one problem only (1.3), none about `page-docs`.
- **Rerun (mine, separate `UX_OUT=../target/rr-out`, 1 worker, no round files touched).** The four
  `/codespec/docs` gating units (the three missing and 1280x720 light) all passed in 3.8 to 4.2 s
  each: 0 failures, 0 overflow, 0 scroll containers, axe `{}`, skip link first and focus moved,
  search and navbar reached, shift 0. So the load reading holds, the three missing units would have
  passed, and no number of the report changes once they are added: gate1/2/5 checked 96, pass 92 /
  96 / 92, waived 4; the ratchet result is unchanged. The files are in `target/rr-out/results`,
  not in the round. The round's `report.md` still says 93.

### 1.3 The ratchet is red and the report does not say so

`compare(baseline, toMeasured(results))`: `worse: page-docs-reference-support layout.overflow 4 >
baseline 0 + 0`. No value is missing and none is better. `report.md` shows "layout.overflow | 0 | 4
| 0 | 0" and "Failures by kind: gate1 overflow, 4, 1, **none**". It does not say the ratchet fails,
because `round.ts` never runs it (T8, which held but was latent in R-07). Here it matters:
`task ux-check` ends with `node cli.ts ratchet` and fails on this tree.

### 1.4 Do the metrics mean what their names say?

- **layout.overflow 4** is real: the span "Evaluator write" (the header label of the Evaluator
  write column) ends 3 to 4 px beyond its parent at all 6 viewports × 2 themes (1024x768: 688 vs
  685; 390x844: 123 vs 120). It is the only overflow on any page. R-07 had 0.
- **Cause: two candidates, not isolated.** The matrix is now 960 px wide in 766 (R-07's reviews: 848),
  and the overflowing label belongs to the Evaluator column, which is new since R-07's tree. Both
  arrived in 0.7.0 (`13312fd5`): the column (data) and UXF-030, which raised the feature id,
  description and code to 0.875rem in `SupportMatrix/styles.module.css`. Nothing under
  `website/src` changed after that commit; later releases changed 27 lines of cell states and notes in
  `support-matrix.json`. The R-07 triage said "the support matrix stays 848 px wide"; the raw results
  say 960. I did not rebuild the site to isolate it.
- **clipped** (collected, never reported, T3): 141 per unit on `reference/support` (67 `span`, 74
  `code`), 162 at 390x844. The R-07 review counted 111 per unit. The page has 128 features (127 in
  R-07). UXF-030 (larger type) is the likely reason for the 30 more cut-off entries, not isolated; UXF-021's title and text still say
  "111 feature descriptions". The report's findings table prints that title.
- **scroll containers 24 → 8** is UXF-029 working: the 5 pages (`static-analysis` 4, `netcheck` 4,
  `crates` 4, `pipeline` 2, `cli` 2 = 16) have none now and the support matrix keeps its 8. The
  per-page table is new (UXF-031) and its "last round" column is "–" because R-07 kept no list; so
  the five pages that dropped to 0 are not named in the report this round.
- **keyboard.misses 8** is 4 failing units (T1). It equals baseline and R-07 and is the waived
  UXF-005, so it hides nothing new.
- **always-visible misses 96** at 768x1024 and 390x844 is `theme toggle:missing`, the collapsed
  navbar (T5), 96 = 48 units × top and bottom.
- **Last round is not the same site.** The column compares R-07's totals (commit `90e3ec1`, 23
  pages, 92 gating units) with 24 pages and 93 measured units (96 expected). Every column of
  "layout.*, keyboard.*, axe.*" is 0 or 8 in both, so the comparison is harmless for these numbers.
  It is not for scroll containers (the page set differs by the codesec page, which has none).
- **The measured tree is not only R-07's change set.** `notes.md` says "R-07's change set and the
  0.7.0 release, with the codesec page". HEAD `24044631` is `0.11.0` plus 2 commits (`git describe`:
  `0.11.0-2-g24044631`). Since the R-07 change set (`13312fd5`, 0.7.0) 19 files under `website/`
  changed: the conformance and support-matrix data, six docs pages (Getting started, the adapter and
  syscalls guides, the cir/1, CLI and Crates references) and the two example SVGs. The Evaluator
  column of `reference/support` (and its paragraph) did not exist in R-07's measured tree
  (`90e3ec1`); it came with 0.7.0, the same commit as the R-07 change set. All five R-07 targets can
  still be read, but the matrix and the page text moved for other reasons too.
- **Did R-07's change set move its targets?**

| # | finding | target | moved |
|---|---|---|---|
| 1 | UXF-029 | gating scroll containers 24 → 8, 5 pages to 0 | yes (24 → 8; 0 on the five pages; both asset tests pass) |
| 2 | UXF-027 | pages stating the archive contents 2 → 1, Status | not read: the Rust assets are outside the round (T12) |
| 3 | UXF-025 | home places missing the C writer 3 → 0 | yes by grep: `product.json` lede, description and "meta" name C; Rust asset not run |
| 4 | UXF-031 | pages named for the scroll containers | yes: the per-page section exists (support, 8) |
| 5 | UXF-030 | ids, descriptions and code at 14 px | yes (asset passes), with the side effects above: overflow 0 → 4 and clipped 111 → 141 |

- **Can the metrics see the operator's goal?** No metric counts content. By `wc -w` on
  `website/docs/**/*.md{,x}`: all pages 31,952 (R-07) → 33,892 (+1,940, +6.1 %); generated
  (`examples/*`, `reference/{cli,cir-1,crates}`) 18,948 → 19,964; hand-written 13,004 → 13,928
  (+924; the new `codesec.md` is 56 lines). Since R-03 (28,735): +5,157 (+17.9 %). The
  operator's notes say another branch edits the prose, so most of that is not this loop's.

## § 2 Findings the report misses

- **The support overflow has no finding and the report does not name its cause.** One failure kind,
  4 units, "waived by **none**". There is no UXF record; the report's Findings table lists none. It
  needs one (S3, layout, the metric `layout.overflow`, cost S: let the matrix header labels wrap or
  widen the column), tied to UXF-021 (the same page, the same matrix).
- **The worst-moment shots for it show nothing wrong.** `shots/fail-page-docs-reference-support-*.jpg`
  are the top of the page ("Language support", the intro, the filter chips). The overflowing header
  is further down, outside the first screen, and the harness outlines axe targets only. The 4
  `fail-page-search-*` shots do show the focused search box (UXF-005).
- **The new "Evaluator" column.** Its filter chip (the 6th language chip in the shot) and a
  paragraph are new since R-07; the matrix now carries 6 languages and scrolls at all desktop
  widths (UXF-021, open, cost M). The overflowing header label is this column's.
- **Failures of the run that are not in the report.** The report says "L1 and `l2-findings.spec.ts`
  in this run: 43 passed, 0 failed", which is true, but 7 Playwright tests failed (3 timeouts, 4
  gate-1 failures) and the report says nothing of the 3 timeouts.
- **The round wrote R-09 first** (known fact 1). I checked the move: `ux/rounds` holds no R-09,
  `trend.md` has one R-08 row and no R-09 row, `R-08/shots` holds the 32 files the report links
  (24 sheets, 8 fail shots), and `round.json` carries id `R-08`, rotating card `task-trace-options`.
  Had the run kept R-09, the rotating label would be `task-add-an-adapter`; the six tasks ran
  either way, so no number is affected. `report.md`'s "Generated by `task ux-round`" is still right,
  but the rewrite used `writeRound` over `.out/round` (`target/rewrite-round.ts`) and the report
  does not say so. `target/round-R08.log` ends with "wrote … R-09/report.md".
- **Carried, unchanged:** the home lede and the C writer (UXF-025, now closed: `product.json` names
  C); UXF-005 (4 units, waived); UXF-019 and UXF-028 (page text, out of this run's scope per
  `notes.md`); search paths "Reference › Reference" in 2 of 6 task traces (decided exception).

## § 3 Tool defects

Since the R-07 review (`90e3ec1`) only `ux/core/round.ts` changed among the tools (UXF-031's
`scrollContainerPages`, `scrollContainerLines`); `results.ts`, `layout.ts`, `ratchet.ts`,
`screen-only.ts`, `specs.ts`, `adapter.ts`, `cli.ts` and the test files `l2-layout`, `l3-screen-only`
and `round-shots` are as they were. T1..T14 of R-07 (`git diff 90e3ec1..HEAD -- ux/core ux/adapter.ts
ux/cli.ts`):

- **T1** holds. `core/results.ts` `toMeasured`: 4 `page-search` units give 8 misses (skipLinkFirst
  and skipMovesFocus both false).
- **T2** holds, did not fire visibly. `core/round.ts` `writeRound` prints `rev-parse --short HEAD`
  with no dirty check. `git status` after the round: `ux/rounds/trend.md` and `R-08/` only, so the
  site and `ux/` sources equal `24044631`. The rewrite (11:08 UTC) read HEAD after the run
  (started 10:41 UTC); both are `24044631`.
- **T3** holds. `adapter.ts` `scrollContainersAllowed` still names `.table-wrapper`; all 8 containers
  are `allowed: false`. `clipped` is collected (564 entries over the gating units, 141 on each
  support unit) and never reported.
- **T4** holds, and it bit. `tests/global-setup.ts` deletes `results/` of the out dir. The three
  missing units cannot be re-measured into `.out/round` without erasing the other 288 files. I
  reran them into a separate `UX_OUT`.
- **T5** holds. `adapter.ts` `alwaysVisible`: 96 + 96 `theme toggle:missing` at 768x1024 and
  390x844.
- **T6** holds. `round.ts` `missing_inputs` are fixed strings; true this round, but silent about the
  3 missing units.
- **T7** holds, latent. `round.ts` `Math.max(0, …)`; no `-1` here (`load` and `afterReady` are 0 in
  all 285 units).
- **T8** holds and now matters. `round.ts` never runs `ratchet.ts` `compare`; the ratchet is red
  here (§ 1.3).
- **T9** holds, minor. `round.ts` `prevId`: the column header does not name R-07. Here it is
  right: `R-07` is the only earlier round with a `round.json`.
- **T10** holds, latent. `core/screen-only.ts` `visibleControls` takes the role from the tag
  before `role`; no task hit it.
- **T11** holds, latent. `runScreenOnly` prefers a strong link in view over one PageDown; the six
  paths are unchanged from R-07.
- **T12** holds (partly fixed in R-07). The round runs L1 and `l2-findings` (43), not the Rust
  assets of `crates/docs`; the report says so. UXF-027 and UXF-025's assets are therefore not in it.
- **T13** holds, latent. `core/specs.ts` `assetResults` is unchanged: no `playwright.json` gives "0
  passed, 0 failed"; `skipped` and `flaky` count as failed; an empty spec counts as passed. All 43
  were `expected`.
- **T14** fixed. `core/round.ts` now writes `scroll_container_pages` and a per-page table. The first
  use has no earlier list ("–" in the last-round column), which the report states.

New:

- **T15** `core/round.ts` `nextRound` (called from `cli.ts` `round()`) counts a directory as a taken
  round number even when it holds only `notes.md`. Input: `ux/rounds/R-08/` with `notes.md` only
  gave `R-09` (`target/round-R08.log`, last lines). `prevId` in the same file already requires a
  `round.json`. The Taskfile text says `-- --round R-nn` re-measures a round, but that reruns the full
  362-test Playwright run (26 minutes) to rewrite a report. Fix direction: count only directories
  with a `round.json`, or let the operator's notes live elsewhere.
- **T16** A unit that fails before its end leaves no result and no trace. `tests/l2-layout.spec.ts`
  writes the result file at the end of the test; `core/results.ts` `gateCounts` counts what exists;
  `core/round.ts` `writeRound` prints "checked 93" with no expected count. Input: the three
  `page-docs` files absent from `ux/.out/round/results` (288 expected, 285 present). The ratchet
  passes the partial scenario (§ 1.2). Fix direction: derive the expected units from scenarios ×
  viewports × themes, print "93 of 96" and list the absent ones; write the result in a `finally`.
- **T17** `playwright.config.ts` `timeout: 60_000` and `retries: 0` are the same in a round as in a
  check. A round unit does three navigations, axe, two scrolls, a keyboard walk of up to 40 Tabs and
  a failure shot; under the slot queue it took 22-49 s at the start and passed 5 s later in the same
  run. The load reading is an inference. Input: `target/round-R08.log` tests
  45 to 52. Fix direction: a round-mode timeout (for example 180 s), or one retry for a timeout only,
  and order `l2-findings` after `l2-layout`.
- **T18** `tests/l2-layout.spec.ts` failure shot: the shot is taken after a fresh `goto`, at scroll
  0, and outlines axe targets only (`sel`). An overflow failure is not outlined or scrolled to.
  Input: `shots/fail-page-docs-reference-support-1366x768-light.jpg`. Fix direction: outline the
  `overflow` element of `measureOverflow` and scroll it into view.
- **T19** `core/round.ts` does not print the Playwright result of the run (362 tests, 7 unexpected,
  3 timed out) or the log's `exit 1`. `round.json` has `regression_assets` only. Input:
  `ux/.out/round/playwright.json` `stats.unexpected` = 7 against a report that shows only the 43
  assets.

## § 4 Caveats to send with the round

1. The measured tree is `24044631` (`0.11.0` + 2 commits), not R-07's change set alone: the support
   matrix (a new Evaluator column) and 6 docs pages changed in the releases since. The ratchet is red on one value
   (`layout.overflow`, `reference/support`, 4 > 0).
2. 93 of 96 gating units were measured. `/codespec/docs` has 1 of 4: 1366x768 light and dark and
   1280x720 dark timed out under load. Rerun alone: all 4 pass in about 4 s each, clean (§ 1.2).
3. `reference/support` now fails hard gate 1 at every viewport (the header "Evaluator write" is 3 to
   4 px wider than its box). No finding exists yet; the cause is the new Evaluator column, UXF-030's 14 px type, or both, and is not
   isolated.
4. The support matrix is 960 px wide in 766 (848 in R-07) and 141 text items per unit are cut off
   (111 in R-07); UXF-021's title still says 111.
5. All five R-07 targets moved or held except #2 (UXF-027), which is a Rust asset outside the round.
   Gating scroll containers 24 → 8 is UXF-029.
6. `keyboard.misses` 8 is 4 failing units on `/codespec/search`, waived by UXF-005.
7. The mobile rows' "always-visible 96" are the collapsed navbar, by design.
8. No metric counts content; the pages gained 1,940 words (+6.1 %) since R-07.
9. The report was rewritten from the R-09 output with `writeRound` after the run wrote the wrong
   directory (T15). Its numbers equal the raw results.

---

**Verdict:** not fit to send as is, default action as above.

**Numbers that differ:** none between the report and the raw files. Not in the report: 3 of 96 gating
units missing; the ratchet is red (`layout.overflow` 4 > 0); 7 failed Playwright tests.

**Tool defects:**
- T1, T3, T4 (bit), T5, T6, T9 hold. T2, T7, T10, T11, T13 hold, latent. T8 holds and matters. T12
  holds (Rust assets outside). T14 is fixed.
- T15 (new): `nextRound` counts a notes-only directory, so the round wrote R-09.
- T16 (new): a timed-out unit leaves no file; "checked 93" has no expected count; the ratchet
  passes the partial scenario.
- T17 (new): one 60 s timeout and no retry for a round under load.
- T18 (new): failure shots show page top and outline only axe targets.
- T19 (new): the report omits the Playwright totals and failures.
