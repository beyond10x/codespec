---
format: aep.planning-md/3
id: approval-record:reader-coverage-wave-2026-11-10
kind: approval-record
status: draft
title: Accept the reader-coverage stories under the standing wave approval
relations:
- decides: epic:reader-coverage
revision: 1
---
# Approval: the reader-coverage wave

## Words

- Timo, 2026-11-10, as a session goal: "readers for quarkus+c+golang+rust close to 100%".
- Timo, 2026-11-10, earlier: "i approve all future 10 waves".
- Timo, 2026-11-10: "keep going, btw do not run any local benchmarks, they must run remotely if at all".

## Decision

- The coordinator drafted `epic:reader-coverage` and eight stories from the goal sentence. The critic panel ran twice (`review-result:rd-*-round-1`, four critics, 33 findings; `rd-acceptance-round-2` and `rd-design-round-2`, 15 findings); every finding was revised, and the last revision was not reviewed again.
- Inferred by the coordinator, not said by Timo: that "100%" is the count of verified features in a reader's `read` column of the support matrix, of the 120 a reader can claim; that "quarkus" is the Java reader and the refusal count of the pinned quickstart; that `cir/1` does not change for this.
- The reader stories take both implementor slots of this repository before the five query and evaluator stories still open, except `story:eval-conformance-runner`, which `story:reader-check-evaluator-step` needs.
- Move to active: `epic:reader-coverage` and its eight stories. `epic:low-level-readers` and its two stories stay draft until the critic panel has read them a second time.
- Commits authorised: as in `approval-record:query-evaluator-waves-2026-11-10`.
