# Wave plan: epic:series-both-ways

Derived 2026-11-06 with `aep plan artifact waves --kind story --status active` (aep 0.79.0):

```
wave 1
  story:rust-reader-loops
wave 2
  story:series-rust-to-go
2 wave(s), 0 collision(s), 0 unassessed
```

| wave | story | objective | scope | state |
|---|---|---|---|---|
| 1 | story:rust-reader-loops | the Rust reader reads the Rust writer's `series` output with 0 refusals | cited: `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`; inferred: `adapters/rust-read/src/lib.rs`, `adapters/rust-read/src/names.rs` | dispatched |
| 2 | story:series-rust-to-go | the Go written from that read joins the `series` harness | cited: `examples/series`, `Taskfile.yml`, `crates/docs/src/series.rs`, `website/docs/examples/series.md`, `CHANGELOG.md` | waits for wave 1 |

Critic panel, round 1: `review-result:sbw-acceptance-round-1`, `sbw-design-round-1`,
`sbw-scope-round-1`, `sbw-parallel-safety-round-1`, all `approve`, 0 findings.

## Wave 1

Approved by the operator: "dispatch the next wave" (2026-11-06). N = 1.

Commits this approval authorises: the opening store commit, 1 unit commit, the merge of the unit
into `wave/series-both-ways`, the closing store commit. The merge into `main` is one pull request
through the bot. Not a tag or release.

| unit | branch | base | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| story:rust-reader-loops | `impl/rust-reader-loops` | `657dde7` | managed id `codespec-w1-loops` | `target/` inside the worktree | session scratch `wave-sbw/rust-reader-loops/scratch` | implementor dispatched |

Integration branch: `wave/series-both-ways`, managed tree `codespec-plan`.

Pre-flight: `/` 87G free; one debug build of the workspace is 330M (`du -sh target` in
`codespec-plan` after `task build`); agent type `aep:implementor`, then `aep:adversary`.
