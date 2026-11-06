---
format: aep.planning-md/3
id: review-result:m1-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
revision: 1
---
needs-revision
story:rust-writer — bullet 1 requires `orders` to write and build as Rust, while bullet 3 requires refusing the goroutine at orders.go:132. Its "unless the story's own decision records a Rust replacement" names a decision the story does not contain, so nobody can tell which bullet governs. — .engineering/planning/story/rust-writer.md:27
story:rust-writer — the Outcome says the crate "passes `clippy -D warnings`", but the first bullet only "builds it" and `cmp`s it, so the clippy claim has no check. — .engineering/planning/story/rust-writer.md:24
story:go-writer — the Outcome says the package "is `gofmt`-clean", but the first bullet only runs `go vet` and `cmp` against the checked-in copy. `adapters-check` gofmts only `adapters/go`, not `examples/orders/out/go`. — .engineering/planning/story/go-writer.md:21
story:go-writer — "A statement or type the writer cannot emit is an error naming its `pos`, exit 1" names no input that triggers it, so the refusal can pass untested on `orders`. — .engineering/planning/story/go-writer.md:25

What I read: 5 ids given and 5 read (the epic and 4 stories) with `aep plan artifact show <id>`. I also ran `aep plan artifact lifecycle story`, read `docs/design.md`, `Taskfile.yml`, `crates/cli/src/main.rs` and `examples/orders/orders.read.cir.json`, and checked the `foreign` node at orders.go:132.

What I could not establish: none. Out of my lane: `codespec write` already exists in `crates/cli/src/main.rs:47`, so no scope gap there. Whether the Rust writer's own decision on the goroutine belongs in the epic's Ambiguities is a design/scope question.

```findings
- file: .engineering/planning/story/rust-writer.md
  line: 27
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "bullet 1 requires orders to write and build as Rust while bullet 3 requires refusing the goroutine at orders.go:132, and its 'unless the story's own decision records a Rust replacement' names a decision the story does not contain, so nobody can tell which bullet governs"
- file: .engineering/planning/story/rust-writer.md
  line: 24
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the crate passes clippy -D warnings but the first bullet only builds it and cmps it, so the clippy claim has no check"
- file: .engineering/planning/story/go-writer.md
  line: 21
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the package is gofmt-clean but the first bullet only runs go vet and cmp against the checked-in copy, and adapters-check gofmts only adapters/go, not examples/orders/out/go"
- file: .engineering/planning/story/go-writer.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the refusal bullet names no input that triggers it, so it can pass untested on orders"
```
