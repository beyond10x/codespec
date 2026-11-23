---
format: aep.planning-md/3
id: story:ux-gate-axe-budget
kind: story
status: draft
title: UX gate units keep the test budget on the support page
summary: The axe scan of the 18-column support matrix makes UX units time out at 60 s; scan once, keep units under 60 s and the step under 120 s
relations:
- informed_by: story:test-budget-existing-suites
revision: 2
---
# Story: ux-gate-axe-budget

## Outcome

Every unit of the website UX gate (`ux/`, Playwright) ends within 60 s, and the UX test step ends
within 120 s, on the shared workstation under load. The step prints its ten slowest units.

## Context

Finding UXF-038 of UI iteration round R-08 (PR #43, 2026-11-24). Since 0.12.0 the support matrix
has 18 role columns. On that page the axe accessibility scan alone takes 36-43 s per L2 unit
(measured at host load 80-110), so the support page's L2 units at 1280x720 time out at the 60 s
Playwright limit. The same scan with the CSS from before R-08 took 42.6-42.9 s against 36.0-36.5 s
after it, so the R-08 CSS change is not the cause.

The local Playwright run with 2 workers took 327 s before the R-08 fix and 493 s after it: the UX
step is over the 120 s step budget independent of this page.

Raising the timeout, adding shards or waiting longer is not a fix (`~/beyond10x/AGENTS.md` § Tests
stay fast).

## Approach (to be confirmed in scoping)

- Scan the support matrix once per build, not per unit and viewport: axe on one rendered matrix
  fixture, keyed by the digest of the matrix data, read by every unit.
- Or restrict axe to the regions a unit changes (`include`/`exclude`), with one full scan per page
  in the `slow` lane.
- Measure the UX step's units before and after; report the ten slowest.

## Done When

- No UX unit runs over 60 s at host load of 100, measured with the step's own slowest-ten list.
- The UX step ends within 120 s, or a further story names the remaining over-budget units.
- The R-08 ratchet values for the support page are measured from the L2 units again.
