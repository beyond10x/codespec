# R-07 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md`, `round.json`, `review-heuristics.md`
and `review-report.md` of R-07.

## Caveats

- No L6 sessions, no questionnaire, no recordings; no L4, L5 or L8: no paid APIs and no people in
  this loop. Findings come from the round's metrics and the two review agents.
- The measured site is commit `90e3ec1` (R-06's change set). `ux/` was dirty with decision R-06's
  answer, UXF-018 closed and the checklist's decided exception; the report prints the bare commit
  (T2). The round ran with 2 Playwright workers because of machine load; no unit failed or flaked.
- The ratchet passes on the measured values: 231 values, none worse, none better, none missing
  (`review-report.md` § 1).
- Scroll containers at the gating viewports rose 18 → 24, a side effect of R-06's UXF-024: three
  more tables scroll sideways and two got wider (both reviews; filed UXF-029).
- No round metric counts words. By `wc -w` the docs pages went 32,035 → 31,952 (−83); +11.2 % over
  R-03 (`review-report.md` § 1).
- Frequency is scenarios affected ÷ the 29 scenarios of the round (23 pages, 6 task cards).
- Harness defects of `review-report.md` § 3 are ranked with the findings. Latent ones (T7, T8, T10,
  T11, T13) have frequency 0. T14 enters the set and is filed as UXF-031.
- Confidence 2 where two sources agree: UXF-029 (both reviews), UXF-025 (both reviews say it holds),
  UXF-030 (the heuristic review's finding and the report review's Status headers), UXF-005 (the
  gate's waived failure and both reviews), UXF-021 (both reviews).

## Did the last change sets move their metrics?

| round | change | target | moved |
|---|---|---|---|
| R-06 | UXF-024 table code and headers | `td code` 12.67 → 14 px, `th` 10.88 → 14 px | yes: 1,664 `td code` on 17 pages and 313 `th` all 14 px; `UXF-024` asset green in the round. It missed the Status table (UXF-030) and widened tables (UXF-029) |
| R-06 | UXF-026 the round runs the assets | asset results in the round 0 → 38 | yes: `report.md` § Regression assets, 38 passed, matches `playwright.json` (20 L1, 18 `l2-findings`) |
| R-06 | UXF-022 claims on several pages | diagram 2 → 1 pages, orders run 2 → 1, counts 1 → 0, "not a language model" 2 → 1 | yes: `each_claim_is_on_one_docs_page` green, grep agrees. The checklist line still failed on the archive contents (UXF-027, new) |
| R-06 | UXF-023 run clause twice | pages saying the run twice 4 → 0 | yes: `the_run_is_stated_once_in_the_header_card` green |
| R-06 | UXF-018 rest (decision no) | `task-cir1-interfaces-generics` stays 2/3/3 | yes: 2/3/3, "Reference" → "Interface" |
| R-03 to R-05 | earlier sets | their assets | held: all 18 `l2-findings` tests and the Rust assets green |

## Formula

| finding | severity | weight | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-029 tables scroll sideways since 14 px code | S3 | 1 | 5/29 = 0.17 | 2 | S = 1 | **0.34** |
| UXF-027 Status calls unreleased C adapters shipped; archive contents differ | S2 | 3 | 3/29 = 0.10 | 1 | S = 1 | **0.31** |
| UXF-025 home lede names Go and Rust writers only | S2 | 3 | 1/29 = 0.03 | 2 | S = 1 | **0.21** |
| UXF-031 (T14) scroll containers a bare total | S3 | 1 | 6/29 = 0.21 | 1 | S = 1 | **0.21** |
| UXF-030 support matrix and Status data below 14 px | S3 | 1 | 2/29 = 0.07 | 2 | S = 1 | **0.14** |
| UXF-028 concept pages describe a Go-and-Rust pipeline | S3 | 1 | 2/29 = 0.07 | 1 | S = 1 | 0.07 |
| UXF-005 search page focuses its box | S3 | 1 | 1/29 = 0.03 | 2 | S = 1 | 0.07 |
| T1 keyboard misses counted twice | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| UXF-019 Go-to-Rust guide prints an 81-line block open | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| UXF-021 support matrix scrolls, descriptions cut | S3 | 1 | 1/29 = 0.03 | 2 | M = 2 | 0.03 |
| T3 scroll-container allowlist misses `.b10x-table-wrap` | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| T2 bare commit over a dirty tree | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| T7, T8, T10, T11, T13 (latent) | S3 | 1 | 0 | 1 | – | 0 |

UXF-029 frequency: concepts/pipeline, reference/cli, reference/crates, concepts/static-analysis,
examples/netcheck. UXF-027: Status, Getting started, Add a reader or writer adapter. UXF-031: the
six pages with a scroll container at a gating viewport.

No S1 finding. The five highest fill the set. Of the queue carried from R-06, UXF-025 is in;
UXF-005, UXF-021, T1, UXF-019 and T3 rank below the new findings and stay open. T3 waits on
purpose: allowlisting `.b10x-table-wrap` would have hidden UXF-029. UXF-019's asset was written
and ran red this round (a `crates/docs` test that fails on a code block over 60 lines outside
`<details>` on a hand-written page: `guides/translate-go-to-rust.md:140`, 81 lines); it left the
tree with the finding.

## Change set R-07

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-029 | `website/src/css/custom.css`: inline code in table cells breaks anywhere (`overflow-wrap: anywhere`), still 14 px | Markdown tables scrolling at a gating viewport on the five pages 3 at 1366x768 and 5 at 1280x720 → 0; gating scroll containers 24 → 8 (the support matrix) | `ux/tests/l2-findings.spec.ts::UXF-029` |
| 2 | UXF-027 | `website/docs/status.mdx`: the page describes this tree, names 0.6.1 as the last release and points to Unreleased in the changelog for what the tree adds; `guides/add-an-adapter.md` links Getting started for the archive contents | pages saying what the archive holds 2 → 1; Status phrases tying shipped to the release 3 → 0 | `crates/docs/src/main.rs::the_status_page_describes_the_tree`, `::each_claim_is_on_one_docs_page` (release archive contents) |
| 3 | UXF-025 | `website/product.json`: lede, description and the "Emit a language" step name the C writer | home places missing the C writer 3 → 0 | `crates/docs/src/status.rs::every_language_list_names_every_adapter` (now reads the lede, the description and the write step) |
| 4 | UXF-031 (T14) | `ux/core/round.ts`: `scroll_container_pages` in `round.json` and a per-page section in `report.md`, against the last round | pages named for the scroll containers 0 → every page | `ux/tests/l1-core.spec.ts::round names the pages of its scroll containers` |
| 5 | UXF-030 | `SupportMatrix/styles.module.css`: feature id, description and its code 0.875rem; `custom.css`: Status headers and chips 0.875rem, sentence case | matrix id 12.8, description 11.52, code 10.94 px → 14; Status headers 10.88, chips 12 px → 14 | `ux/tests/l2-findings.spec.ts::UXF-030` |

No design decision this round. UXF-027 had a choice (Status describes the tree, or drops the C
entries until a release carries them); the tree is the reversible default and matches the home page
and the overview, which already say "this tree".

Words on the pages edited: status.mdx 322 → 341, add-an-adapter.md 963 → 948 (+4 in all).

Facts removed, each with where it stays:

| page | fact | stays on |
|---|---|---|
| guides/add-an-adapter.md | each release's archive holds `codespec`, the Go and Rust readers and writers and their runners | Getting started § Install (linked) |
| guides/add-an-adapter.md | each release's archive holds the C writer and `codespec-run-c` | not true of 0.6.1; the release workflow `.github/workflows/release-build.yml` packs them for the next release, and Status points to Unreleased in the changelog |
| status.mdx | "The changelog records every user-visible change" | the changelog's own first line; Status still links it |

## Build and gate

Each asset ran red before its change and green after: `UXF-029` (2 tests) and `UXF-030`
(Playwright), the two `round names the pages of its scroll containers` L1 tests, and
`the_status_page_describes_the_tree`, `each_claim_is_on_one_docs_page` and
`every_language_list_names_every_adapter` (Rust). The site build gives no sideways-scrolling
Markdown table on the five pages at either gating viewport; the support matrix stays 848 px wide.
`task ux-check` is green: 142 Playwright tests passed. In its results, scroll containers at the
gating viewports went from 24 to 8, all of them the support matrix's. The ratchet reads 29
scenarios with no value worse and no improvement untaken, so `ux/baseline.json` is unchanged:
scroll containers are measured, not ratcheted.
`cargo test -p codespec-docs`: 43 passed, 2 failed, both the branch's known staleness
(`reference/crates.md`, `intrinsic-sorted-keys/case.go`).
