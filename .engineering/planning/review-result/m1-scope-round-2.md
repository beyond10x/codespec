---
format: aep.planning-md/3
id: review-result:m1-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
- reviews: epic:go-and-rust-writers-with-harness
revision: 1
---
needs-revision
story:harness — the epic states "No trace calls `Audit`: its output depends on map order and goroutine scheduling", but the harness body never says the trace generator leaves `Audit` out of its command set, and the Rust crate has no `Audit` to call. So no item claims this constraint, and a generator that emits it breaks the Rust run. State it in the Outcome or Acceptance. — .engineering/planning/epic/go-and-rust-writers-with-harness.md:41-43

The round-1 `story:rust-writer` findings are resolved. Revision 3 of the epic and `story:rust-writer` now state the same `Audit` refusal, and the clippy requirement is gone.

Promises extracted from the epic: 10. Traced to an item: 9; the tenth ("No trace calls `Audit`") is the finding above.
- Passes and `codespec pass`: `story:passes-and-facts`.
- Go writer, with the `foreign` goroutine printed verbatim: `story:go-writer`.
- Rust writer, with `Audit` listed as a refusal: `story:rust-writer`.
- Writers refuse a `foreign` statement in another language: `story:go-writer` acceptance 3 and `story:rust-writer` outcome.
- `crates/harness` and `codespec harness`: `story:harness`.
- Drivers under `examples/orders/drivers/`: `story:harness`.
- 200 × 300 byte-equal, with seed and line named on failure: `story:harness`.
- `task check` runs the harness: `story:harness` acceptance 1.
- Planted one-token change fails: `story:harness` acceptance 2.
- Go → `cir/1` → Go: `story:go-writer`.

What I read: 5 artifacts, using `aep plan artifact show <id>` for the epic and the four stories (all at the revisions named in the round-2 message), plus `aep plan artifact graph`. This is a from-scratch reading; the round-1 files in the worktree were also at hand for the `foreign` line numbers.

What I could not establish:
- Out of my lane (acceptance): the fixed-point acceptance in `story:go-writer` and the "map order does not leak" acceptance in `story:harness` are checkable as written.
- Out of my lane (parallel safety): the shared `check` list in `Taskfile.yml` is documented as a one-line conflict resolved at merge.

```findings
- file: .engineering/planning/epic/go-and-rust-writers-with-harness.md
  line: 41
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic states 'No trace calls Audit' but story:harness never says the trace generator excludes Audit, so no item claims the constraint and a trace that emits it fails the Rust run; state it in the harness Outcome or Acceptance"
```
