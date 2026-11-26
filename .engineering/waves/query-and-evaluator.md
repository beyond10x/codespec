# Wave plan: query and evaluator

Derived 2026-11-10 (AEP skill 0.26.0, `aep` 0.79.0). The store held no accepted story; the seven
below were drafted from `docs/query-language.md` § 9 (items 1 to 3) and `docs/evaluator.md` § 11
(stories 1 to 4), both "Status: proposed".

Approval: `approval-record:query-evaluator-waves-2026-11-10` (Timo: "select the next 7 work items
... dispatch up to 2 opus agents at once. one wave each repo, then cut new release"; "i approve all
future 10 waves").

Critic panel: `review-result:qe-{acceptance,design,scope,parallel-safety}-round-1` and `-round-2`.
Round 1: four `needs-revision`, 14 findings, all revised. Round 2: design and scope approve;
acceptance 6 findings and parallel-safety 1 finding, all revised, not reviewed a third time.

Integration branch `wave/query-and-evaluator`, forked from `main` at `4204699`, in the managed
worktree `codespec-wave-1110-int`. Agents: `aep:implementor` and `aep:adversary`, model opus, at
most two units at once. Units leave their changes uncommitted; the coordinator commits each unit
as the bot on `impl/<story>`, merges it, lets `cargo build` rewrite `Cargo.lock`, runs
`task docs-generate`, and runs `task check` once per merged pair. After the seventh story:
`task check`, push, merge to `main`, then the release of `AGENTS.md` § Release.

Check path: the coordinator's reading of the Scope sections plus the parallel-safety critic; one
typed scope entry per story, all inferred. No `aep:story-scoper` ran.

| order | slot | story | unit branch | worktree | scratch | state |
|---|---|---|---|---|---|---|
| 1 | Q | story:query-core | impl/query-core | `<trees>/qe-query-core` | `<scratch>/cs/query-core` | implemented (store) |
| 1 | E | story:eval-core | impl/eval-core | `<trees>/qe-eval-core` | `<scratch>/cs/eval-core` | implemented (store) |
| 2 | Q | story:query-paths-properties | impl/query-paths-properties | `<trees>/qe-query-paths` | `<scratch>/cs/query-paths` | implemented (store) |
| 2 | E | story:eval-conformance-runner | impl/eval-conformance-runner | `<trees>/qe-eval-conform` | `<scratch>/cs/eval-conform` | implemented (store) |
| 3 | E | story:eval-handlers-traces | impl/eval-handlers-traces | `<trees>/qe-eval-handlers` | `<scratch>/cs/eval-handlers` | implemented (store) |
| 4 | E | story:eval-fault-plans | impl/eval-fault-plans | `<trees>/qe-eval-faults` | `<scratch>/cs/eval-faults` | implemented (store) |
| 5 | alone | story:cir1-defer-receiver-root | impl/cir1-defer-receiver-root | `<trees>/qe-cir1-defer` | `<scratch>/cs/cir1-defer` | dispatched |

`<trees>` is `~/.local/state/worktree/trees/b10x/codespec`; `<scratch>` is the coordinating
session's scratch directory; each build directory is `target/` inside its worktree.

Shared between the two slots, resolved by the coordinator at merge: `crates/cli/src/lib.rs`,
`crates/cli/src/main.rs`, `crates/cli/Cargo.toml`, the `members` line of `Cargo.toml`, `Cargo.lock`,
`CHANGELOG.md`, `AGENTS.md`, `Taskfile.yml`, the generated site reference.

## Backlog (not in this wave)

- The eight earlier drafts: `story:cir-patterns` (superseded by `story:query-core`),
  `story:cir-rewrite`, `story:defect-patterns`, `story:instrumentation`, `story:mutation-testing`,
  `story:performance-hotspots`, `story:taint-flows`, `story:unhandled-branches`.
- Not yet drafted: query design § 9 items 4 to 6, evaluator design § 11 stories 5 to 8.

## Wave 5, 2026-11-26

The last wave of this plan: `story:cir1-defer-receiver-root`, alone (N = 1). Its four
`depends_on` stories are implemented; `aep plan artifact blocked` prints "nothing is blocked".
The owner's standing approval of 2026-11-10 (`approval-record:query-evaluator-waves-2026-11-10`)
and "dispatch next /aep:wave" (2026-11-26) start it. AEP skill 0.26.0, `aep` 0.80.0.

`aep plan artifact waves --kind story --status active` for this story:
`wave 1  story:cir1-defer-receiver-root (inferred)`; no collision names it. Scope: one typed entry
(`crates/model/src/lib.rs`, inferred) and the Scope section, inferred. The other accepted stories
(`story:reader-coverage-floor`, `story:reader-check-stops-when-reached`,
`story:reader-java-to-error-lookup-else`) are not on this plan and wait for the next one.

Integration branch `wave/cir1-defer-receiver-root` from `main` at 7724ef8c, managed tree
`wave-next-1126`. Unit: branch `impl/cir1-defer-receiver-root`, managed tree `qe-cir1-defer`, build
directory `target/` inside it, scratch `<scratch>/cs/cir1-defer`. Agents: `aep:implementor`, then
`aep:adversary`. Builds through `slot` (route to the build node; a node tool gap reruns once with
`--here`).

Commits this wave makes: this page, one unit commit, its merge into the integration branch, the
closing store commit (evidence and the move to implemented), and a pull request to `main` that
the operator session merges. No tag, no release.

After this wave every wave of the plan is delivered.

## The wave grows, 2026-11-26

The owner, 05:18 CET: "now, move in way bigger /aep:wave and deliver PRs with more stories +
gap-closures at once". One integration branch (`wave/cir1-defer-receiver-root`), one pull request.
Scopes from `aep:story-scoper`, one per story.

| order | story | unit branch | managed tree | scratch | state |
|---|---|---|---|---|---|
| A | story:cir1-defer-receiver-root | impl/cir1-defer-receiver-root | qe-cir1-defer | `<scratch>/cs/cir1-defer` | dispatched |
| A | story:reader-check-stops-when-reached (C and Java; the Rust part shipped in e2cff037) | impl/stop-reached-c-java | qe-stop-reached | `<scratch>/cs/stop-reached` | dispatched |
| A | story:diff-unknowns-summary | | | | waits for acceptance |
| B | story:diff-unknown-at-the-incomplete-function (after diff-unknowns-summary: same files in crates/diff) | | | | waits for acceptance |
| B | story:go-std-effect-summaries-common (after cir1-defer: its reproducer is Go reader output and `defer` changes) | | | | waits for acceptance |
| B | story:test-budget-existing-suites (after cir1-defer: adapters/go, crates/eval, crates/query) | | | | waits for acceptance |
| C | story:reader-coverage-floor (last and alone: its floors are the merged support matrix) | | | | planned |
| - | story:reader-java-to-error-lookup-else | none | | | already on main (8a863ed1, 2026-11-10); closed with an observed conformance run |

The four stories marked "waits for acceptance" are drafts. The acceptance (decision D5, option A)
was refused by the coordinator's permission check; they join when the owner moves them to active.

Commits this wave makes: the pages, one commit per unit, its merge into the integration branch,
the closing store commit, one pull request to `main`. No tag, no release.
