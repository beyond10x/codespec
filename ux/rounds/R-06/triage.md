# R-06 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md`, `round.json`, `review-heuristics.md`
and `review-report.md` of R-06.

## Caveats

- No L6 sessions, no questionnaire, no recordings; no L4, L5 or L8: no paid APIs and no people in
  this loop. Findings come from the round's metrics and the two review agents.
- The measured tree is the clean commit `647f0d8` (R-05's change set). The round ran with 2
  Playwright workers because of machine load; no unit failed or flaked.
- The ratchet passes on the measured values: 231 values, none worse, none better, none missing
  (`review-report.md` § 1).
- No round metric counts words or screens. By `wc -w` R-05 cut 54 words; the site is still +11.5 %
  over R-03 (`review-report.md` § 1).
- Frequency is scenarios affected ÷ the 29 scenarios of the round (23 pages, 6 task cards).
- Harness defects of `review-report.md` § 3 are ranked with the findings. A latent one (T2, T7, T8,
  T10, T11: present in code, no scenario affected this round) has frequency 0. Only T12 enters the
  set; it is filed as UXF-026.
- Confidence 2 where two sources agree: UXF-018 and UXF-021 (both reviews name them this round),
  UXF-005 (the gate's waived failure and both reviews), UXF-026 (the report review's T12 and R-05's
  hand check of the R-04 targets).

## Did the last change sets move their metrics?

| round | change | target | moved |
|---|---|---|---|
| R-05 | UXF-020 sidebar group names | 10.88 → 14 px | yes: 14 px, sentence case (Playwright in both reviews; `UXF-020` asset green) |
| R-05 | UXF-016 language lists | places missing an adapter 10 → 0 | yes for the places the test reads; the home page's lede still names Go and Rust writers only (UXF-025, new) |
| R-05 | UXF-017 Start here claims | orders run, trace counts and ESS oracle on one Start here page each; use-cases 891 → 816, vision 755 → 723 words | yes: test green, 816 and 723. The same claims stay on non-Start-here pages (UXF-022, new) |
| R-05 | UXF-015 cir/1 reference index first | `task-cir1-interfaces-generics` 6/10/7 → 2/3/3 | yes: 2/3/3 in `round.json` ("Reference" → "Interface"); screen-only totals 19/27 → 15/20 |
| R-05 | UXF-018 (part) overview names and header card | old names 2 → 0, Start here pages without a card 1 → 0 | yes: test green; the open part goes to decision R-06 |
| R-03, R-04 | earlier sets | their assets | held: every L2 finding test and Rust asset green |

## Formula

| finding | severity | weight | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-024 table code 12.7 px, headers 10.9 px | S3 | 1 | 16/29 = 0.55 | 1 | S = 1 | **0.55** |
| UXF-026 (T12) the round runs no regression asset | S3 | 1 | 8/29 = 0.28 | 2 | M = 2 | **0.28** |
| UXF-022 claims on several pages | S3 | 1 | 5/29 = 0.17 | 1 | S = 1 | **0.17** |
| UXF-023 run clause twice on four pages | S3 | 1 | 4/29 = 0.14 | 1 | S = 1 | **0.14** |
| UXF-018 rest: navbar sections, "Reference › Reference" | S3 | 1 | 7/29 = 0.24 | 2 | L = 4 | **0.12** |
| UXF-025 home lede names Go and Rust writers only | S2 | 3 | 1/29 = 0.03 | 1 | S = 1 | 0.10 |
| UXF-005 search page focuses its box | S3 | 1 | 1/29 = 0.03 | 2 | S = 1 | 0.07 |
| UXF-021 support matrix scrolls, descriptions cut | S3 | 1 | 1/29 = 0.03 | 2 | M = 2 | 0.03 |
| T1 keyboard misses counted twice | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| UXF-019 Go-to-Rust guide prints whole files | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| T3 scroll-container allowlist, clipped text unreported | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| T2, T7, T8, T10, T11 (latent) | S3 | 1 | 0 | 1 | – | 0 |

UXF-024 frequency: the 16 docs pages whose tables hold inline code.
UXF-026 frequency: the scenarios whose findings have an asset the round did not run. UXF-018's open
part costs L: it needs a navigation change the ratchet sees (decision.md).

No S1 finding. The five highest fill the set; UXF-018's part waits for its decision and blocks
nothing. UXF-025 is next in line.

## Change set R-06

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-024 | `website/src/css/custom.css`: inline code in table cells and headers, and table headers, 0.875rem; headers in sentence case | `td code` 12.67 → 14 px, `th` 10.88 → 14 px | `ux/tests/l2-findings.spec.ts::UXF-024` |
| 2 | UXF-026 | `ux/core/specs.ts` (spec pattern per mode), `ux/playwright.config.ts`; round mode runs L1 and `l2-findings`; `ux/core/round.ts` writes `regression_assets` and a report section | asset results in the round 0 → every L1 and `l2-findings` test (dry run on this tree: 38) | `ux/tests/l1-core.spec.ts::round runs the regression assets` |
| 3 | UXF-022 | the overview links the pipeline diagram; The harness links Getting started's run; Vision's M1 row and design rule drop the counts and "language model" and link the overview | pages holding the diagram 2 → 1, the orders run 2 → 1, the trace counts 1 → 0, "not a language model" 2 → 1 | `crates/docs/src/main.rs::each_claim_is_on_one_docs_page` |
| 4 | UXF-023 | the first paragraph of Use cases, Getting started and the two guides keeps only what the header card does not say | pages saying the run twice 4 → 0 | `crates/docs/src/main.rs::the_run_is_stated_once_in_the_header_card` |
| 5 | UXF-018 (rest) | none built: decision R-06, default no | `task-cir1-interfaces-generics` stays 2/3/3 with no | decision.md |

Words on the pages edited: 6,727 → 6,644 (−83).

Facts removed, each with where it stays:

| page | fact | stays on |
|---|---|---|
| index.md | the pipeline diagram | Readers, passes and writers (linked from the overview) |
| concepts/harness.md | the `codespec harness examples/orders` run and its output line | Getting started § Run the harness (linked) |
| vision.md | M1 "byte-equal on 200 × 300 traces" | Getting started § Run the harness; the M1 row keeps "byte-equal under the harness" |
| vision.md | "No step is a language model" | the overview § What it is not (linked from the rule) |
| use-cases.md, getting-started.md, translate-go-to-rust.md, map-a-repository-to-syscalls.md | installed codespec, clone or empty directory, release, Go version, output pasted from that run | each page's header card (`source:`); Use cases' card gains "in a clone" |

## Build and gate

Each built asset ran red before its change and green after: `UXF-024` and the two `round runs the regression assets` L1 tests (Playwright), `each_claim_is_on_one_docs_page` and `the_run_is_stated_once_in_the_header_card` (Rust). A dry run of `writeRound` on the gate's results lists 38 L1 and `l2-findings` tests, all passing. `task ux-check` is green: 137 Playwright tests passed; the ratchet reads 29 scenarios with no value worse and no improvement untaken, so `ux/baseline.json` is unchanged. `cargo test -p codespec-docs`: 42 passed, 2 failed, both the branch's known staleness (`reference/crates.md`, `intrinsic-sorted-keys/case.go`). R-07 measures each change's target; its round runs the assets itself.
