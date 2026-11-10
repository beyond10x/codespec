---
format: aep.planning-md/3
id: story:reader-coverage-open-after-0-8-0
kind: story
status: draft
title: 'Reader coverage: what the 0.8.0 wave left open'
relations:
- decomposes: epic:reader-coverage
revision: 1
---
# Story: reader coverage, what the 0.8.0 wave left open

## Outcome

Each item below is either landed with its reader report number or closed with a written reason,
so the `read` columns of Go, Rust, C and Java stand at every feature a reader of that language
can claim.

## State at 0.8.0

Read from `website/data/conformance/reader-<lang>.json` at the release: Go 115, Rust 113, C 105,
Java 105 claimed and passing, 0 failing.

## Open items

- **C and Java `stmt.foreign`.** Their case sources exist (`conformance/cases/stmt-foreign/c/case.c`,
  `.../java/Case.java`). The claim was not made because the evaluator called a function by its
  `cir/1` name; `codespec-run-cir` resolves a call name as Go spells it since 0.8.0. Retry: run
  the two cases, claim what passes. Expected C +1, Java +1; not measured.
- **Externs outside Go** (`story:reader-check-externs`). The reader check has no way to run a
  module whose externs are not Go's. Estimated from the gap analysis: Rust +3, Java +3, C +2; not
  measured.
- **Java `stmt.raise` in a kit case.** The Java reader reads a raise only in a project read, in a
  function an entry names; a kit case is a package read with no entry.
- **Go `expr.zero`, `type.unit`, `intrinsic.sorted_keys`.** Three features the Go reader does not
  claim.
- **Per-language denominators** in `story:reader-coverage-floor`: the floor counts against one
  number for all readers, while some features cannot exist in a language (channels in Java).
- **`fact.*` for readers.** Facts are computed by `codespec pass`, never claimed by a reader; the
  reports count them as untested. They should be not applicable.
- **Adversary review not done** for two units that shipped in 0.8.0 on their own tests only:
  the Java `expr.to_error` and `stmt.lookup_else` forms, and the reader check's
  `--stop-when-reached` unit (`story:reader-review-java-forms-and-lazy-stops`).

## Acceptance

- Every item above is a landed claim with its report line, or a line in the adapter's doc saying
  why the feature cannot be claimed.
- `task check` exits 0.
