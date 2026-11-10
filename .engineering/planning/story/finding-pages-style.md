---
format: aep.planning-md/3
id: story:finding-pages-style
kind: story
status: draft
title: Finding pages in the animated procedure style
summary: Finding pages follow the animated pipeline, playback and trace-table look of the reference procedure page.
revision: 1
---
# Story: Finding pages in the animated procedure style

## Outcome

A page that presents the findings of a hunt over `cir/1` documents follows one reference look,
given by the operator on 2026-11-10 through a peer session: the animated procedure page built for
a hunt over a Go SIP library. The reference files are outside this repository; the peer session's
message names them.

## Acceptance

- The page is one self-contained HTML file: no CDN, no network request.
- Dark palette (cyan, violet, amber, green, rose), a light palette on light-mode systems; severity
  is shown by colour and a text label together.
- Section 1, an animated pipeline, left to right: source, reader, `cir/1` documents, hunts with
  their named detectors, candidate rows, confirmed in source, failing reproducer test, severity
  bins. One dot per detector output row; counters tick to the real numbers; rejected rows drop at
  the confirm step.
- Section 2, per-finding playback, highest severity first: detector, query output line,
  `file:line`, test, evidence, severity; play/pause, previous, step, speed, a finding picker and
  clickable id chips.
- Section 3, a static trace table of all findings, with a re-runnable recipe at the bottom.
- `prefers-reduced-motion` shows the final state without animation.
- Every count is read from a file; a derived count is labelled as derived.

## Scope

- Not scoped. No wave has taken this story.
