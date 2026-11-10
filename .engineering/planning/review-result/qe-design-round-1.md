---
format: aep.planning-md/3
id: review-result:qe-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (query and evaluator)
relations:
- reviews: story:query-core
- reviews: story:query-paths-properties
- reviews: story:cir1-defer-receiver-root
- reviews: story:eval-core
- reviews: story:eval-conformance-runner
- reviews: story:eval-handlers-traces
- reviews: story:eval-fault-plans
revision: 1
---
needs-revision
story:query-core — its acceptance requires all 8 built-in checks, including `lock-without-unlock`, to match `codespec check` on the hit and miss fixtures. That check needs the `lock_released` property (§ 5.3) and a deferred Unlock matched to the Lock's receiver (§ 8). The miss fixture has `defer c.mu.Unlock()`. This story refuses properties and path patterns by name. `story:query-paths-properties` supplies properties and `story:cir1-defer-receiver-root` supplies `defer` and `recv_var`, and both are scheduled after this story. The acceptance reads what two later stories produce, and no edge could record that without a cycle. Re-scope it to the checks expressible with § 5.1 and § 4.2 alone. Move the `lock-without-unlock` equivalence acceptance to `story:cir1-defer-receiver-root`, the last in slot Q, which already depends on `story:query-paths-properties`. — .engineering/planning/story/query-core.md:31, crates/check/tests/fixtures/lock-without-unlock/miss/miss.go:28, docs/query-language.md:190-205 and :298-306

**What you read:** 7 artifacts via `aep plan artifact show` on each id, plus `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`. I also read `docs/query-language.md` §§ 4 to 9, `docs/evaluator.md` §§ 3, 8, 9 and 11, `story:builtin-defect-checks` and its lock-without-unlock fixtures.

**Graph and acceptance reads:**
- I walked 23 edges, including those to `story:cir-patterns`, the two epics and `epic:whole-repo-to-syscalls`, which are outside the set. I found no cycle among the `depends_on` edges.
- Slot E is a linear chain, but each item's Scope line names the shared crate beside its `after` edge, so I did not report it as serialising.
- I traced 12 acceptance reads to a producer in the set. Edges record 11 of them, but the lock-without-unlock read is the one that cannot be recorded.

**What I could not establish:**
- `aep plan artifact validate` printed no output that I saw. The graph and validate commands wrote their output to a file, and I read only the graph portion for the edges above.
- Out of my lane, for parallel safety. Slots Q and E both list `crates/cli/src/lib.rs`, `main.rs`, `Cargo.toml`, `CHANGELOG.md`, the generated `website/docs/reference/{cli,crates}.md`, `crates/docs/src/status.rs` and `AGENTS.md` in their scope. No edge orders them across the slots.
- Out of my lane, for acceptance. `story:cir1-defer-receiver-root` adds a kit case but names no check that `codespec conform --lang cir` still passes. Whether `story:eval-conformance-runner` lands before or after it is open.
- Out of my lane, for scope. `docs/evaluator.md` § 11 stories 5 to 8 are not in this set. The `cir/1` additions of § 9 beyond `defer` (panic, recover, string/[]byte conversion) are also not in this set. That looks deliberate.

```findings
- file: .engineering/planning/story/query-core.md
  line: 31
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance requires all 8 built-in checks, including lock-without-unlock, to match codespec check on the hit and miss fixtures, but that check needs the lock_released property (§ 5.3) and a deferred Unlock matched to the Lock's receiver (§ 8, and the miss fixture uses defer c.mu.Unlock()), which story:query-paths-properties and story:cir1-defer-receiver-root produce after this story and which this story refuses by name; re-scope the acceptance to the checks expressible with § 5.1 and § 4.2 and move the lock-without-unlock equivalence to story:cir1-defer-receiver-root, the last item in slot Q"
```
