# R-05 triage

Applied by rule (ui-iteration-loop § 3.3), from `report.md`, `round.json`, `review-heuristics.md`
and `review-report.md` of R-05.

## Caveats

- No L6 sessions, no questionnaire, no recordings; no L4, L5 or L8: no paid APIs and no people in
  this loop. Findings come from the round's metrics and the two review agents (screenshots,
  Playwright probes, `file:line`).
- The measured tree is `34edbf7` plus, uncommitted, a regenerated `website/docs/reference/cli.md`
  (a merge had left the `codespec query` usage block unclosed, so the site did not build) and the
  page scenario `ux/scenarios/page-docs-guides-read-a-quarkus-project`; `report.md` and the trend
  row say `34edbf7` only (`review-report.md` T2).
- The tree measured failed the ratchet before any change of this round: `task-cir1-interfaces-generics`
  2/3/3 → 6/10/7 actions/looks/steps, and the Quarkus page had no baseline (`review-report.md` § 2).
  The report prints neither (T8).
- No round metric counts words or screens (`review-report.md` § 1). The site grew 28,735 → 32,089
  words since R-03 (+11.7 %, two new pages). Content targets below are tests and `wc -w`.
- `reference/crates.md` and some `website/data/conformance/*` files are stale on this branch from
  other work and are left so; `task docs-generate` rewrote them and they were restored.
- Tool defects T1–T11 (`review-report.md` § 3) are not fixed in this round: a fix to the screen-only
  user (T10, T11) or to the metric reduction (T1) would move the same numbers the change set moves.
- Frequency is scenarios affected ÷ the 29 scenarios of the round (23 pages, 6 task cards).
- Confidence 2 where two sources agree: UXF-015 (the ratchet value and both reviews), UXF-016 (the
  heuristic review's five lists and the report review's out-of-date language list on the cir/1
  reference), UXF-017 (the heuristic review's repeated runs and the report review's +11.7 % words),
  UXF-005 (the gate failure and both reviews).

## Did the last change sets move their metrics?

| round | change | target | moved |
|---|---|---|---|
| R-03 | UXF-012, 007, 010, 014, 006 | the five assets in `ux/tests/l2-findings.spec.ts` and `crates/docs/src/status.rs` | held: green in R-05. Mobile axe serious+critical 42 → 0 at 768x1024 and 390x844 (hamburger target size, R-03 report review) |
| R-04 | UXF-013 fold long code | blocks over 60 lines open on generated pages 3 → 0 | held: `no_generated_page_shows_a_long_code_block_open` green; longest open block 47 lines (`review-heuristics.md` checklist line 3) |
| R-04 | UXF-009 one page per claim | vision 869 → 755 words, status 364 → 322 | held: 755 and 322 at `34edbf7`. Checklist line 1 fails again on content added since (UXF-017) |
| R-04 | UXF-011 one name per page | pages whose title, label and H1 differ 6 → 0 | held: 0 on 21 docs pages (`every_page_has_one_name` green). Checklist line 2 fails on navbar and search paths (UXF-018) |
| R-04 | UXF-008 netcheck worked once | hand-written pages working the netcheck example 2 → 1 | held: test green; Use cases runs the netcheck read again (UXF-017) |

No R-03 or R-04 change has gone two rounds without moving its target (rule 4).

## Formula

| finding | severity | weight | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-020 sidebar group names at 10.9 px | S3 | 1 | 21/29 = 0.72 | 1 | S = 1 | **0.72** |
| UXF-016 five different language lists | S2 | 3 | 6/29 = 0.21 | 2 | M = 2 | **0.62** |
| UXF-017 Use cases reruns Getting started; claims on several pages | S3 | 1 | 8/29 = 0.28 | 2 | S = 1 | **0.55** |
| UXF-015 cir/1 interfaces and generics 2 → 6 actions | S2 | 3 | 2/29 = 0.07 | 2 | S = 1 | **0.41** |
| UXF-018 navbar names and "Reference › Reference" | S3 | 1 | 6/29 = 0.21 | 1 | S = 1 | **0.21** |
| UXF-005 search page focuses its box on load | S3 | 1 | 1/29 = 0.03 | 2 | S = 1 | 0.07 |
| UXF-019 Go-to-Rust guide prints whole files open | S3 | 1 | 1/29 = 0.03 | 1 | S = 1 | 0.03 |
| UXF-021 support matrix scrolls sideways, descriptions cut | S3 | 1 | 1/29 = 0.03 | 1 | M = 2 | 0.02 |

No S1 finding. The five highest fill the set. UXF-015 is in by priority; it also clears the ratchet
failure the tree had before this round. UXF-005, 019 and 021 wait.

## Change set R-05

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-020 | `website/src/css/custom.css`: level-1 sidebar category labels 0.875rem, no uppercase, normal letter spacing (decision.md, default yes) | category label 10.88 px → 14 px | `ux/tests/l2-findings.spec.ts::UXF-020` |
| 2 | UXF-016 | `product.json` meta, the overview's sentence, adapter row and "What it is not" item, the pipeline adapter table (Java, Groovy readers, C writer; the C reader's language is Rust); Status gains "C reader conforms to the kit" and "C writer conforms to the kit" (`crates/docs/src/status.rs`) | places missing an adapter 10 → 0 | `crates/docs/src/status.rs::every_language_list_names_every_adapter` |
| 3 | UXF-017 | Use cases § 1 links Getting started for the orders run instead of pasting it; the overview drops the trace counts and links the harness page, the home harness card drops them; Vision's ESS item links the overview's Neighbours | pages restating the orders run 2 → 1, the trace counts 4 → 1, the ESS oracle 2 → 1; use-cases 891 → 816 words, vision 755 → 723 | `crates/docs/src/main.rs::each_start_here_claim_is_on_one_page` |
| 4 | UXF-015 | `crates/docs/src/cir.rs`: the definitions index comes before the Module table | Interface and TypeParam links in the first screen 0 → 2 at both gating viewports; `task-cir1-interfaces-generics` 6/10/7 → baseline 2/3/3 | `ux/tests/l2-findings.spec.ts::UXF-015`; the ratchet on `task-cir1-interfaces-generics` |
| 5 | UXF-018 | the overview links "codespec CLI" and "Showcase: greeter" by their sidebar names and gets the header card (`lede`), so Start here counts 1 of 3 | old names on the overview 2 → 0; Start here pages without a header card 1 → 0 | `crates/docs/src/main.rs::the_overview_names_pages_as_the_sidebar_does`; `ux/checklist.md` line 2 |

UXF-018 is built in part. Not done, with the reason: the navbar keeps "Documentation" and
"Reference" as section names, and search keeps "Reference › Reference" (the search plugin writes
the active navbar item, then the sidebar group). A reference index page for the navbar would add a
click to the cir/1 task, and dropping the navbar's active state would lose the section cue. The
finding stays open for that part.

Facts removed, each with where it stays:

| page | fact | stays on |
|---|---|---|
| use-cases.md § 1 | the commands `read`, `pass`, `write --lang rust`, `cat refusals.json`, `harness` on orders, and their output (`Service.Audit` at `orders.go:132`, 200 × 300, 79,990 lines, byte-equal) | Getting started (steps on `try/`, `refusals.json`, the harness run) |
| use-cases.md § 1 | the `diff -r` run showing the written crate equals `examples/orders/out/rust` | the sentence stays on Use cases § 1 ("The crate equals `examples/orders/out/rust`") |
| index.md | "agree on 200 traces of 300 commands each" | Getting started (the harness run and the sentence after it), The harness |
| product.json | the harness card's "200 seeded traces of 300 commands" | Getting started, The harness |
| vision.md § Later | ESS would supply a skeleton, its conformance suite an equivalence oracle; a direction, not used today | the overview § Neighbours (which vision now links) |
| index.md | "the subset of Rust that `cir/1` covers with the Rust reader on `syn`" | the overview's adapter row (`codespec-read-rust` in Rust on `syn`), Language support |

## Build and gate

Each asset ran red before its change and green after: the three Rust tests (`every_language_list_names_every_adapter`, `each_start_here_claim_is_on_one_page`, `the_overview_names_pages_as_the_sidebar_does`) and the three Playwright tests (`UXF-015` at both gating viewports, `UXF-020`). `task ux-check` is green on the built tree: 134 Playwright tests passed; the ratchet reads 29 scenarios with no value worse and no improvement untaken. The baseline gains the nine values of `page-docs-guides-read-a-quarkus-project`, all 0; no value changed. `task-cir1-interfaces-generics` measured 2/3/3 again ("Reference" → "Interface"). R-06 measures each change's target.
