# Wave plan: error propagation in the Rust writer

Width: one unit; `story:rust-writer-error-propagation` is the only story not implemented, and its
vocabulary, case, reader and writer changes cannot land apart without a red conformance report.

| wave | story | objective | state |
|---|---|---|---|
| 1 | story:rust-writer-error-propagation | `sem.error.propagate`: a fallible call whose error the caller returns is written as Rust instead of refused; the Go reader reads `err != nil` | dispatched |

Critic panel: round 1 `review-result:erp-acceptance-round-1`, `erp-design-round-1`,
`erp-scope-round-1`, `erp-parallel-safety-round-1`; round 2 `review-result:erp-panel-round-2`;
round 3 parallel-safety.

Housekeeping in the same branch: `epic:adapter-conformance` and `epic:groovy-grails-reader` moved
to implemented, every story under them being implemented.
