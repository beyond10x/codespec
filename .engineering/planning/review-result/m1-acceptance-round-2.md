---
format: aep.planning-md/3
id: review-result:m1-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
revision: 1
---
needs-revision
story:harness — the Outcome promises the output names "the seed, the implementation and the first differing line", but the planted-change test "asserts exit 1 with the seed" only. The implementation name and the line, which the epic's Outcome also promises, are checked by nothing. — .engineering/planning/story/harness.md:29

All four round-1 findings are resolved:
- The unchecked clippy claim is dropped from story:rust-writer's Outcome.
- story:go-writer's task `examples-go` now requires `gofmt -l` to print nothing.
- A `foreign` statement with `lang: "rust"` is the named refusal case in story:go-writer.
- The conflict between "builds Rust" and "refuse the goroutine" is replaced by the `refusals.json` listing of `Audit` at `orders.go:132`.

What I read: 5 of 5 ids given (the epic at revision 3 and the four stories), each with `aep plan artifact show <id>`. I also grepped `story/harness.md` for the line number.

What I could not establish: none. Out of my lane: the `Taskfile.yml` `check`-list conflict between the stories is now declared in their Scope sections, and judging it belongs to the parallel-safety critic.

```findings
- file: .engineering/planning/story/harness.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the seed, the implementation and the first differing line but the planted-change test only 'asserts exit 1 with the seed', so the implementation name and the line are checked by nothing"
```
