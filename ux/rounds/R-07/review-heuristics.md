# R-07 heuristic review

Verdict: 10 pass, 5 partial, 1 fail

Checklist (`ux/checklist.md`): 2 pass, 1 fail.

Inputs: `report.md`, every contact sheet in `shots/` (2 gating viewports × 2 themes), the four `fail-*`
shots, the task cards, `notes.md` and the page sources. There are no recordings this round, so the
evidence is screenshots and `file:line`. Extra shots come from Playwright (`ux/node_modules`) against
`website/build` on 127.0.0.1 and are named `shots/review-*.png`. Measurements are at 1366x768 light
unless stated. The site judged is the build; the worktree also holds uncommitted tests in
`crates/docs` for UXF-019 and UXF-025, whose fixes are not in the build.

| # | heuristic | verdict | evidence |
|---|---|---|---|
| 1 | One next thing | pass | "Get started" is in the home page's first screen at both gating viewports (`shots/sheet-page-home.jpg`). Every docs page ends in a pager |
| 2 | State in words | pass | Status chips say "SHIPPED" and the page defines the terms (`shots/sheet-page-docs-status.jpg`). Support bars say "96 % 115 ✓ · 0 ○ / 120" and "no writer" (`shots/review-support-1440x900-light.png`) |
| 3 | Nothing required hidden | partial | Gate 2 has 0 misses (`report.md`). The support matrix still scrolls sideways at 1440x900 (848 px in 821, legend 829 in 821) and 51 descriptions are clipped (`shots/review-support-1440x900-light.png`): open UXF-021 holds. New: tables on five pages scroll sideways at the gating viewports, up from three, so the last column is cut until scrolled (`shots/review-pipeline-table-1280x720-light.png`). UXF-029 |
| 4 | Stable layout | pass | `layout_shift.after_ready` 0 and load 0 on every page (`report.md`). Nothing moves between frames in any sheet |
| 5 | Provenance of every value | fail | Status says it describes release 0.6.1 and that shipped means "in the release", then lists the C reader and C writer as shipped; neither is in 0.6.1 (`shots/review-status-c-writer-1366x768-light.png`, `website/docs/status.mdx:12,14`, `CHANGELOG.md:5,123-124`). Getting started and Add an adapter disagree on what the archive holds (`getting-started.md:18-19`, `guides/add-an-adapter.md:119-121`). UXF-027. The home lede still says "writes Go or Rust" above "Go, Rust and C writers" (`shots/sheet-page-home.jpg`): open UXF-025 holds. The cir/1 model says "a Go package today", and the pipeline page's Writers covers Go and Rust only (`concepts/cir-1.md:11-12`, `concepts/pipeline.md:4,75-86`). UXF-028 |
| 6 | Errors cheap | pass | A wrong page costs one action back. All 6 tasks are reached, gate 4 6/6, every task at its baseline actions (`report.md`) |
| 7 | Words the user can say | pass | Title, H1, active sidebar item and last breadcrumb agree on all 21 docs pages (Playwright). Footer labels match the sidebar (`shots/sheet-page-search.jpg`). Navbar "Documentation"/"Reference" and search "Reference › Reference" are the decided exception (decision R-06, no) |
| 8 | The reader's time respected | partial | The Go-to-Rust guide still prints its 81- and 57-line files open, 12.8 screens (`guides/translate-go-to-rust.md:19-77,140-222`): open UXF-019 holds. The site did not get shorter: 27,011 words, 218.5 screens over 21 docs pages (R-06: 27,077 and 215.0) |
| 9 | Keyboard and pointer parity | partial | Home: Tab runs skip link → wordmark → navbar → theme. `/codespec/search` starts with focus in the input and the first Tab lands on the footer's "Overview" (`shots/fail-page-search-1366x768-light.jpg`): open UXF-005 holds |
| 10 | Recognition over recall | pass | task-cir1-interfaces-generics 2/3/3, "Reference" → "Interface"; the definitions list is in the first screen (`shots/sheet-page-docs-reference-cir-1.jpg`) |
| 11 | Calm by default | pass | No motion, banners or alerts on any page (all sheets) |
| 12 | One state vocabulary | partial | shipped/decided/planned are used one way in words, but "shipped" means "in release 0.6.1" on Status and "in this tree" for the C adapters on Status and in the pipeline page's adapter table (`concepts/pipeline.md:37`). UXF-027 |
| 13 | A wrong cue recovered in one action | pass | Breadcrumbs, sidebar and search paths give one-click recovery on every docs page (all `sheet-page-docs-*.jpg`) |
| 14 | Unmistakable end state | pass | On every docs page the active sidebar item, the last breadcrumb and the H1 match (Playwright on all 21 pages) |
| 15 | Readable in every theme; key data ≥ 14 px | partial | axe contrast 0 serious in both themes (`report.md`). Markdown table code and headers are 14 px on every page (UXF-024 moved). In the support matrix the feature ids are 12.8 px, their descriptions 11.5 px and code 10.9 px; Status headers are 10.9 px and chips 12 px (`shots/review-support-1440x900-light.png`). UXF-030 |
| 16 | Focus follows the work | pass | Only the current group is open and its active item is in the sidebar's first screen on every sheet (all `sheet-page-docs-*.jpg`) |

## Checklist

| line | verdict | evidence |
|---|---|---|
| Each claim is stated on one page and linked from the others. | fail | The four named claims hold: the diagram is on `concepts/pipeline.md:12` only, the orders run on `getting-started.md:177` only, "not a language model" on `index.md:34` only. But the release archive's contents are stated on two pages and disagree (`getting-started.md:18-19`, `guides/add-an-adapter.md:119-121`). UXF-027 |
| A page has one name in the navbar, sidebar, H1, title, footer and search. | pass | Title, sidebar, H1, breadcrumb and footer agree on all 21 pages. Navbar and search differ only as the decided exception names |
| No code block over 60 lines is open by default on a generated page. | pass | The longest open blocks on the generated examples are 47 lines (greeter `lib.rs`), 40 (call trees) and 35 (netcheck); the longer ones sit in details (`examples/greeter.md:73,264`, `examples/netcheck.md:29`). The hand-written Go-to-Rust guide is outside the line: UXF-019 |

## Did R-06's changes move their targets?

| finding | target | seen here |
|---|---|---|
| UXF-024 | table code and headers 14 px | moved in markdown tables on all 21 pages; three more tables now scroll sideways and two grew wider (UXF-029); the support matrix and Status kept their sizes (UXF-030) |
| UXF-022 | one claim, one page | moved for the four named claims; the line fails on the archive contents (UXF-027) |
| UXF-023 | the run stated once | moved: the first paragraph on Use cases, Getting started and the two guides says only what the card does not (`shots/sheet-page-docs-use-cases.jpg`) |
| UXF-026 | the round reports the assets | moved: `report.md` lists 38 passed, 0 failed |
| UXF-018 | closed by decision R-06 (no) | the checklist names the exception; line 2 passes |

## Structure, for the round's goal

- Words (article text including code, Playwright `innerText`, same method as R-06): 27,011 over 21
  docs pages, 218.5 screens at 1366x768. R-06: 27,077 and 215.0. Words fell by 66; screens rose by
  3.5, mostly from taller tables at 14 px (cir/1 reference 32.7 → 34.0, codespec CLI 21.5 → 21.8).
- The longest pages are the generated references: cir/1 (3,650 words, 34.0 screens), Language
  support (3,329) and the CLI (3,041, 21.8 screens). The longest hand-written pages are Static
  analysis (14.0 screens), Go-to-Rust (12.8) and the syscalls guide (12.2).
- First visit: home (3.8 screens) → Overview (4.1) → Use cases (5.5) → Getting started (10.4).
- Scroll containers at the gating viewports rose from 18 to 24 (`round.json`): all six new ones are
  tables widened by the R-06 font change (UXF-029).

## Findings filed

| id | title | severity | class | sessions | cost |
|---|---|---|---|---|---|
| UXF-027 | Status describes release 0.6.1 but calls the C reader and C writer shipped, and two pages disagree on what the release archive holds | S2 | copy | none (no sessions in R-07) | S |
| UXF-028 | Two concept pages still describe a Go-and-Rust pipeline: "a Go package today", and Writers covers Go and Rust only | S3 | copy | none | S |
| UXF-029 | Since table code is 14 px, tables on five pages scroll sideways at the gating viewports, up from three | S3 | layout | none | S |
| UXF-030 | The support matrix and the Status table set their data below 14 px: feature ids 12.8 px, code 10.9 px, descriptions 11.5 px | S3 | layout | none | S |

Open records that still hold, not refiled: UXF-005, UXF-019, UXF-021, UXF-025. The records pass
`findingProblems` in `ux/core/findings.ts` (30 records, 0 problems).
