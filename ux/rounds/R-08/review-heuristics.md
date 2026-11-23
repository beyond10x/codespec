# R-08 heuristic review

Verdict: 10 pass, 5 partial, 1 fail

Checklist (`ux/checklist.md`): 3 pass, 0 fail (one pass carries a note, UXF-033).

Inputs: `report.md`, every contact sheet in `shots/` (2 gating viewports x 2 themes), the eight `fail-*`
shots, the task cards, `notes.md`, the page sources and the changelog. There are no recordings this
round (no L6), so the evidence is screenshots and `file:line`. Extra shots come from Playwright
(`ux/node_modules`) against `website/build` on 127.0.0.1 and are named `shots/review-*.png`;
measurements are at 1366x768 light unless stated. The site judged is the build of `main` at
`24044631`. Another branch edits `website/docs/**/*.md` now: findings carry `fix_in`.

| # | heuristic | verdict | evidence |
|---|---|---|---|
| 1 | One next thing | pass | "Get started" is in the home page's first screen at both gating viewports and themes (`shots/sheet-page-home.jpg`); task-install reached in 1 action (`report.md`) |
| 2 | State in words | pass | Chips say "SHIPPED"; support bars say "98% 125 ✓ · 0 ○ / 128", "no writer", "no reader" (`shots/review-support-matrix-1366x768-light.png`) |
| 3 | Nothing required hidden | partial | Gate 2 has 0 misses. Gate 1 now fails: the Evaluator write bar label ends 7 px past the article edge (`shots/review-support-matrix-1366x768-light.png`, `shots/fail-page-docs-reference-support-1366x768-light.jpg`): UXF-034. Open UXF-021 holds: the legend (941 px in 766) and the matrix (960 in 766) both scroll sideways, so the C and Evaluator columns are cut at 1366x768, and 67 of 128 descriptions end in an ellipsis (`shots/review-support-rows-1366x768-light.png`). No other page has a scroll container at 1366x768 (24 in R-07, 8 in R-08 = this page only: UXF-029 holds) |
| 4 | Stable layout | pass | `layout_shift.after_ready` 0 and load 0 on every page (`report.md`); no frame differs in any sheet |
| 5 | Provenance of every value | fail | Status says the C reader and writer are "under Unreleased in the changelog" at release 0.11.0; Unreleased is empty and 0.7.0 shipped them (`shots/review-status-1366x768-light.png`, `website/docs/status.mdx:12-14`, `CHANGELOG.md:5,294`): UXF-032. The install page lists the archive as Go and Rust only; it also holds the C writer, evaluator and runners (`website/docs/getting-started.md:18-19`, `.github/workflows/release-build.yml:82`): UXF-033. Open UXF-028 holds: the cir/1 model still says "a Go package today" (`website/docs/concepts/cir-1.md:11-12`) and the pipeline page's Writers has Go and Rust only (`shots/review-pipeline-1366x768-light.png`, outline). Status shows "As of 2026-11-08" under release 0.11.0 (`website/data/status.json:2`) |
| 6 | Errors cheap | pass | All 6 tasks reached, gate 4 6/6, every task at its baseline actions (`report.md`) |
| 7 | Words the user can say | pass | Document title, H1, sidebar label, breadcrumb and footer label agree on all 22 docs pages (Playwright title/H1 scan; the footer's "Showcase: greeter" is the sidebar label). Navbar "Documentation"/"Reference" and search "Reference > Reference" stay the decided exception (R-06) |
| 8 | The reader's time respected | partial | The Go-to-Rust guide is still 12.8 screens with two whole files open: open UXF-019 holds. Longest pages by scroll height: cir/1 reference 37.2 screens, CLI 22.7, Series 17.3, Static analysis 14.0 (generated references, content is length) |
| 9 | Keyboard and pointer parity | partial | Docs page Tab order: skip link, wordmark, navbar, theme toggle, search, sidebar (Playwright). `/codespec/search` still starts in the input and the first Tab lands on the footer (`shots/fail-page-search-1366x768-light.jpg`): open UXF-005 holds (waived, 4 units) |
| 10 | Recognition over recall | pass | task-cir1-interfaces-generics 2/3/3 via "Reference" > "Interface"; search paths read as page names (`report.md`) |
| 11 | Calm by default | pass | No motion, banner or alert on any page (all sheets) |
| 12 | One state vocabulary | partial | "Shipped" is defined once (code in this tree, held by a named check) and used so, but Status then calls the C adapters unreleased while the pipeline table lists them "shipped" and the archive holds the C writer: UXF-032 |
| 13 | A wrong cue recovered in one action | pass | Breadcrumb, sidebar group and search paths give one-click recovery on every docs page (`shots/sheet-page-docs-*.jpg`) |
| 14 | Unmistakable end state | pass | Active sidebar item, last breadcrumb and H1 match on every docs page; the navbar section is underlined (`shots/sheet-page-docs-reference-support.jpg`, `shots/sheet-page-docs-status.jpg`) |
| 15 | Readable in every theme; key data >= 14 px | partial | axe 0 serious in both themes (`report.md`). The UXF-030 targets hold (Status chips and headers 14 px). New: the "shipped" chips in the pipeline page's adapter table are 10.4 px, the smallest text in the article (`shots/review-pipeline-1366x768-light.png`): UXF-035. The matrix's coverage labels (12.8 px), role labels (12.0 and 11.5 px) and "n/a" symbols (10.9 px) stay below 14 px in the widget UXF-030 fixed for feature ids only. Chrome is below 14 px by design: navbar links 13.4 px, footer links 13.1 px, pager labels 11.2 px |
| 16 | Focus follows the work | pass | Only the current group is open and its active item is in the sidebar's first screen on every sheet (`shots/sheet-page-docs-*.jpg`); the Reference and Project groups open on their pages |

## Checklist

| line | verdict | evidence |
|---|---|---|
| Each claim is stated on one page and linked from the others. | pass | Diagram, run, "no language model" and the archive's contents each sit on one page (`website/docs/guides/add-an-adapter.md:121` links the install page). Note: the archive list on that one page is short (UXF-033); the line tests placement, not accuracy |
| A page has one name in the navbar, sidebar, H1, title, footer and search. | pass | Title, sidebar, H1, breadcrumb and footer agree on all 22 pages; navbar and search differ only as the decided exception names |
| No code block over 60 lines is open by default on a generated page. | pass | No fenced block over 60 lines sits outside `<details>` on greeter, series, call-trace or netcheck (awk over `website/docs/examples/*.md`); `crates/docs` holds the test |

## Did R-07's changes move their targets?

| finding | target | seen here |
|---|---|---|
| UXF-029 | no markdown table scrolls sideways | moved: scroll containers 24 -> 8, all on the support page (`report.md`); the other five pages have none at 1366x768 (Playwright) |
| UXF-030 | matrix feature ids and Status chips/headers 14 px | moved for feature ids (13.6 px and up in the matrix scan) and Status; the chips of another page and the matrix's own labels are UXF-035 |
| UXF-025 | home lede names the C writer | moved: "writes Go, Rust, C" in the first screen (`shots/sheet-page-home.jpg`) |
| UXF-027 | status and archive agree | partly: the version and "in this tree" definition hold; the sentence about Unreleased went stale (UXF-032), the archive list is short (UXF-033) |
| UXF-031 | the round names pages of its scroll containers | moved: `report.md` lists the support page, 8 |

## Structure, for the round's goal

- 22 docs pages, about 222 screens by scroll height at 1366x768 (R-07: 218.5 by the text method, not
  directly comparable). First visit: home 3.9 screens, Overview 4.2, Use cases 5.5, Getting
  started 10.4.
- The one hard-gate regression, UXF-034, comes from the new Evaluator column, not from a UI change.
- Queue items outside the scope of this review: UXF-028 and UXF-019 are page text; UXF-005 is the
  search plugin.

## Findings filed

| id | title | severity | class | fix_in | sessions | cost |
|---|---|---|---|---|---|---|
| UXF-032 | Status says the C reader and C writer are under Unreleased, but Unreleased is empty and they shipped in 0.7.0 | S2 | copy | text | none (no sessions in R-08) | S |
| UXF-033 | Getting started says the archive holds the Go and Rust readers and writers; it also holds the C writer, the evaluator and three runners | S3 | copy | text | none | S |
| UXF-034 | The support matrix's coverage bars run past the article edge: with the Evaluator column the 12 bars no longer fit the grid (gate 1, 4 units) | S3 | layout | ui | none | S |
| UXF-035 | The "shipped" chips in the adapter tables are 10.4 px, while the same chips on Status are 14 px | S3 | layout | ui | none | S |

Open records that still hold, not refiled: UXF-005 (fix_in upstream or ui: wrap the plugin's search
page), UXF-019 (text), UXF-021 (ui), UXF-028 (text).
