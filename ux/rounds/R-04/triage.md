# R-04 triage

R-04 builds the change set R-03's triage left for the next round: UXF-008, 009, 011 and 013, the
content reduction the operator asked for. No new measure was taken before it; the findings, their
priorities and their evidence are R-03's (`../R-03/triage.md`, `../R-03/review-heuristics.md`).

## Caveats

- No L6 sessions, no L4/L5, no new review agents. The change set's targets are the regression
  assets below, each red before its change; the every-merge harness (`task ux-check`) is the gate.
- No round metric measures amount of content (`../R-03/review-report.md` § 2). The movements below
  are word counts (`wc -w`) and open code lines on generated pages, measured on the tree before and
  after.

## Did the last change sets move their metrics?

| round | change | target | moved |
|---|---|---|---|
| R-03 | UXF-012, 007, 010, 014, 006 | the five assets in `ux/tests/l2-findings.spec.ts` and `crates/docs/src/status.rs` | green since R-03; `task ux-check` green on this tree |

## Change set R-04

| # | finding | change | target (before → after) | regression asset |
|---|---|---|---|---|
| 1 | UXF-013 | `folded` in `crates/docs/src/showcase.rs`: a code block over 60 lines is a details element with its title and line count; greeter and netcheck use it | code lines open on greeter 409 → 132, on netcheck 138 → 59; blocks over 60 lines open 3 → 0 | `crates/docs/src/main.rs::no_generated_page_shows_a_long_code_block_open` |
| 2 | UXF-009 | vision.md's pipeline section, status.mdx's "Not provided" and the home page's ESS section become links to the page that holds them | vision 869 → 755 words, status 364 → 322, product.json 596 → 545 | `ux/checklist.md` line 1 |
| 3 | UXF-011 | `front_matter` writes one name as title, sidebar label and H1; the index is "Overview"; navbar 6 → 4 items left of GitHub; footer 14 → 7 links, each by its sidebar label | pages whose title, label and H1 differ 6 → 0 | `crates/docs/src/main.rs::every_page_has_one_name` |
| 4 | UXF-008 | the concept page's netcheck section is one link to the guide's step 6; the guide's step 7 links to the concept page's measurement | hand-written pages that work the netcheck example 2 → 1; static analysis 2,306 → 2,213 words, the guide 1,773 → 1,724 | `crates/docs/src/main.rs::the_netcheck_example_is_worked_on_one_hand_written_page` |

Hand-written pages edited: 5,926 → 5,628 words.

Facts removed, each with where it stays:

| page | fact | stays on |
|---|---|---|
| concepts/static-analysis.md | the netcheck table: `accept4`, 1 `rta` / 2 `cha` candidates, `FixedProber.Probe` dead | the syscalls guide § 6 and netcheck |
| concepts/static-analysis.md | the client reaches `connect` "through 15 calls" | the drawn path on Call trees § netcheck; the count itself is not stated elsewhere |
| guides/map-a-repository-to-syscalls.md | 228 imports, 27 s, 3.6 s, 74 and 75 syscalls | Static analysis § Measured |
| vision.md | the pipeline table | Readers, passes and writers; The harness |
| status.mdx | "Not provided" (no network I/O, no language model, no ESS) | the overview § What it is not |

Not done: netcheck and Call trees stay two generated pages (UXF-008); the header card of
hand-written pages and its "n of 4" count, which `@beyond10x/docs-system` renders (UXF-011).

The first gate run had the navbar "Reference" open the CLI page: task-cir1-interfaces-generics went 2 -> 3 actions, 3 -> 5 looks (ratchet fail). "Reference" opens the cir/1 reference; the ratchet is clean.
