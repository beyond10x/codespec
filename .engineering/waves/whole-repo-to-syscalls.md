# Wave plan: epic:whole-repo-to-syscalls

Derived 2026-11-08 with `aep plan artifact waves --kind story --status active` (aep skill 0.26.0).
The verb placed 17 waves; `story:jvm-readers-error-propagation` (wave 1) and
`story:rust-reader-error-propagation` (wave 2) belong to earlier plans and are not in this one.
703 collisions: every story shares `crates/model/src/lib.rs`, the Go reader or `crates/effects/`
with its neighbours, so each wave holds one story.

Approval: `approval-record:whole-repo-to-syscalls-waves` (Timo: "stories ... then implement all
/wave"). Integration branch `feat/whole-repo-to-syscalls`, stacked on `feat/externs-and-effects` (PR #10),
in a managed worktree. Commits: one per
unit on `impl/<story>`, its merge, one store commit per wave. No push, no release.

Critic panel: `review-result:whole-repo-{acceptance,design,scope,parallel-safety}-r1` and `-r2`. Round 2:
3 approve; acceptance found 2 on `story:call-trace-view` (fixed), parallel-safety found 1 on
`story:syscall-barrier` (edge to `story:dispatch-over-set` added).

## Change of plan, 2026-11-08

Timo: "move fast in one go, not by running huge gates after every single small change". Units now
run in parallel batches (up to 4 worktrees, limited by disk), each with targeted checks only; the
coordinator merges them into `feat/whole-repo-to-syscalls`, resolving the shared-file conflicts, and runs
`task build && task check` once per batch. foreign-calls, link-modules, dispatch-over-set and
syscall-barrier ran as one unit (`impl/syscall-path`).

| batch | story | unit branch | state |
|---|---|---|---|
| 0 | story:go-reader-whole-package | integration tree (a6ad45f, 91be6c1) | implemented |
| 1 | story:cir1-arrays-floats | impl/cir1-arrays-floats (70a8909) | implemented |
| 1 | story:cir1-extern-types | impl/cir1-extern-types (1d21098) | implemented |
| 1 | story:cir1-func-values | impl/cir1-func-values (75368dd) | implemented |
| 1 | story:cir1-module-vars | impl/cir1-module-vars (c0a6a91) | implemented |
| 1 | story:foreign-calls, story:link-modules, story:dispatch-over-set, story:syscall-barrier | impl/syscall-path (827b232) | implemented; batch gate on 19e4db3, exit 0 |
| 2 | story:cir1-interfaces | impl/cir1-interfaces (21defa7), merge 4320124 | implemented |
| 2 | story:cir1-embedded-fields | impl/cir1-embedded-fields, merge a7b4531 | implemented |
| 2 | story:cir1-variadic | impl/cir1-variadic (5cfa60d), merge 477e16f | implemented |
| 2 | story:reachability (+ rapid type analysis for dispatch precision) | impl/reachability (014993d), merge 0caea74 | implemented; batch gate on 4320124, exit 0 |
| 3 | story:cir1-generics | impl/cir1-generics (fb5a5fd), merge 626c526 | implemented |
| 3 | fix: 5 standard-library packages refused by strict reading (found by library-syscall-map) | impl/strict-read-fixes, merge 750791b | merged |
| 3 | story:library-syscall-map | impl/library-syscall-map (7332deb), merge 584f4fa, regenerated 5639c01 | implemented; batch gate on 626c526, exit 0 |
| 4 | story:call-trace-view | impl/call-trace-view, merge 5dcc2fb | implemented; batch gate on 5dcc2fb, exit 0 on the second run |

Outside the plan, same branch: `docs/static-analysis` (public docs, merged 62351d8).
