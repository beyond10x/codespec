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
| 1 | Q | story:query-core | impl/query-core | `<trees>/qe-query-core` | `<scratch>/cs/query-core` | dispatched |
| 1 | E | story:eval-core | impl/eval-core | `<trees>/qe-eval-core` | `<scratch>/cs/eval-core` | dispatched |
| 2 | Q | story:query-paths-properties | impl/query-paths-properties | `<trees>/qe-query-paths` | `<scratch>/cs/query-paths` | planned |
| 2 | E | story:eval-conformance-runner | impl/eval-conformance-runner | `<trees>/qe-eval-conform` | `<scratch>/cs/eval-conform` | planned |
| 3 | E | story:eval-handlers-traces | impl/eval-handlers-traces | `<trees>/qe-eval-handlers` | `<scratch>/cs/eval-handlers` | planned |
| 4 | E | story:eval-fault-plans | impl/eval-fault-plans | `<trees>/qe-eval-faults` | `<scratch>/cs/eval-faults` | planned |
| 5 | alone | story:cir1-defer-receiver-root | impl/cir1-defer-receiver-root | `<trees>/qe-cir1-defer` | `<scratch>/cs/cir1-defer` | planned |

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
