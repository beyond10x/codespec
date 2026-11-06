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

## Plan widened, 2026-11-06

The operator asked for more work in parallel ("just dispatch some more work"), a Java and a Grails
2.5.6 reader, adapter conformance and a support matrix. New epics `adapter-conformance` (6
stories), `java-reader` (2 stories) and `groovy-grails-reader` (draft, follows `java-reader`).
Re-derived with `aep plan artifact waves --kind story --status active`:

```
wave 1
  story:feature-vocabulary
  story:java-reader-javac
  story:rust-reader-loops (inferred)
wave 2
  story:conformance-cases
  story:java-example-ledger
  story:runners-and-manifests
  story:series-rust-to-go
wave 3
  story:codespec-conform
  story:support-matrix
wave 4
  story:conformance-kit-release
collision: story:codespec-conform story:conformance-cases crates/conformance
collision: story:codespec-conform story:feature-vocabulary crates/cli/src/lib.rs
collision: story:codespec-conform story:feature-vocabulary crates/cli/src/main.rs
collision: story:conformance-kit-release story:series-rust-to-go CHANGELOG.md
collision: story:feature-vocabulary story:runners-and-manifests crates/model/src/lib.rs
4 wave(s), 5 collision(s), 0 unassessed
```

Every collision is between stories in different waves. `Taskfile.yml` is the coordinator's in every
wave: units send patches.

## Wave 1

Approved by the operator: "dispatch the next wave" (2026-11-06). N = 1.

Commits this approval authorises: the opening store commit, 1 unit commit, the merge of the unit
into `wave/series-both-ways`, the closing store commit. The merge into `main` is one pull request
through the bot. Not a tag or release.

| unit | branch | base | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| story:rust-reader-loops | `impl/rust-reader-loops` | `657dde7` | managed id `codespec-w1-loops` | `target/` inside the worktree | session scratch `wave-sbw/rust-reader-loops/scratch` | implementor green (22→0 refusals, cases 10→20); adversary pass 1 red: 5 findings, 4 introduced, 1 undecided (decided: introduced), recorded as `review-result:adversary-rust-reader-loops-pass-1`; correction round 1 green (reader 18→23, examples-check and examples-inventory exit 0); adversary pass 2: 3 blockers, 1 warning, recorded as `review-result:adversary-rust-reader-loops-pass-2`; correction round 2 green, verified by the coordinator (model+cli tests, clippy, fmt, docs check exit 0); committed `f0f1b35`, merged `f8f5e70` |
| story:feature-vocabulary | `impl/feature-vocabulary` | `90bb66f` | managed id `codespec-w1-feature-vocabulary` | `target/` inside the worktree | session scratch `wave-sbw/feature-vocabulary/scratch` | implementor green (cases 9→17); adversary pass 1 red: 3 findings, recorded as `review-result:adversary-feature-vocabulary-pass-1`; correction round 1 green (model features 6→12, adversary 3 of 3); adversary pass 2: 1 warning (reader not strict, 6 red), recorded as `review-result:adversary-feature-vocabulary-pass-2`; correction round 2 green, verified by the coordinator (model+cli tests, clippy, fmt, docs check exit 0); committed `f0f1b35`, merged `f8f5e70` |
| story:java-reader-javac | `impl/java-reader-javac` | `90bb66f` | managed id `codespec-w1-java-reader-javac` | `target/` inside the worktree | session scratch `wave-sbw/java-reader-javac/scratch` | implementor green (0→16, JDK 21 with --release 17; the JDK 17 here is a JRE); adversary pass 1: 5 blockers, 2 warnings, 1 note, recorded as `review-result:adversary-java-reader-javac-pass-1`; correction round 1 green (24 of 24); adversary pass 2: 2 blockers, 3 warnings, 1 note, recorded as `review-result:adversary-java-reader-javac-pass-2`; correction round 2 dispatched, coordinator verifies |

Integration branch: `wave/series-both-ways`, managed tree `codespec-plan`.

Pre-flight: `/` 87G free; one debug build of the workspace is 330M (`du -sh target` in
`codespec-plan` after `task build`); agent type `aep:implementor`, then `aep:adversary`.

Wave 1 closed: `task check` on `7d318a8` exit 0; 3 stories implemented; one PR to `main` through the bot.
Carried to `story:java-example-ledger`: writers render `sorted_keys`, `conv` to `int32` in Rust, and a
field and a method sharing a name in Go.
