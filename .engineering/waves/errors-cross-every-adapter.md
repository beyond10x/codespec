# Wave plan: errors cross every adapter

Width: three units, every story of `epic:errors-cross-every-adapter` that is not implemented; the
Groovy reader is out of the epic's scope. The third joined after its panel passed
(`review-result:ifinit-panel`).

| wave | story | objective | state |
|---|---|---|---|
| 1 | story:rust-reader-error-propagation | the Rust reader reads `?` on a fallible call of the same crate as the propagation form and claims `sem.error.propagate` | merged |
| 1 | story:jvm-readers-error-propagation | the Java reader claims `sem.error.propagate` with a kit case in Java; `examples/ledger` throws from `Account.post` again | merged |
| 1 | story:go-reader-if-init-error-check | the Go reader hoists `if err := f(); err != nil` when the meaning holds, so it reaches the `?` path | merged |

The two units are disjoint in source; both regenerate `conformance/reader/*`,
`website/data/conformance/*.json` and `website/data/support-matrix.json` and add a `CHANGELOG.md`
entry. The second merge keeps both entries and runs `task docs-generate` on the merged head.

Critic panel: passed in three rounds before activation (the epic's `review-result` artifacts).
