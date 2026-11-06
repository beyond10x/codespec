---
format: aep.planning-md/3
id: review-result:m1-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1 (M1 decomposition)
relations:
- reviews: story:passes-and-facts
- reviews: story:go-writer
- reviews: story:rust-writer
- reviews: story:harness
- reviews: epic:go-and-rust-writers-with-harness
revision: 1
---
needs-revision
story:rust-writer — orders contains the `foreign` goroutine (`Audit`, `orders.go:132`), so refusing it (acceptance 3) and writing a building `out/rust/` (acceptance 1) cannot both hold. The body does not say which holds, so either the epic's "Go → `cir/1` → Rust without a hand edit" is silently dropped or the "unless … records a Rust replacement" clause adds a hand-made Rust translation of the goroutine that the epic excludes. The body must pick one and say which part of the epic outcome it leaves unclaimed. — .engineering/planning/story/rust-writer.md:27 against .engineering/planning/epic/go-and-rust-writers-with-harness.md:14,40-41
story:rust-writer — "passes `clippy -D warnings`" and the summary's "clippy-clean" are a quality bar on the generated crate that the epic never asks for (it asks for byte-equal behaviour); drop it or trace it to a parent sentence — .engineering/planning/story/rust-writer.md:7,18

Promises extracted from the epic: 9. Traced to an item: 8; the ninth (Rust output of orders given the refused `foreign` goroutine) is the first finding.

- Passes in Rust and `codespec pass`: `story:passes-and-facts`.
- Go writer on `go/ast` and `go/printer`: `story:go-writer`.
- Rust writer `codespec-write-rust`: `story:rust-writer`.
- `crates/harness` and `codespec harness`: `story:harness`.
- `examples/orders/drivers/`: `story:harness`.
- 200 × 300 byte-equal with seed and line named on failure: `story:harness`.
- `task check` runs the harness: `story:harness` acceptance 1.
- Planted one-token change fails: `story:harness` acceptance 2.
- Go → `cir/1` → Go: `story:go-writer`.

What I read: 5 artifacts, with `aep plan artifact show <id>` for the epic and the four stories. I also read `aep plan artifact graph`, `docs/design.md`, `examples/orders/source/orders.go` and `Taskfile.yml`. No other epic's stories claim any of this.

What I could not establish:
- Out of my lane (acceptance): `story:harness` acceptance 1 relies on `task check` running the harness, but its Scope never lists a `Taskfile.yml` change. `story:passes-and-facts` lists one.
- Out of my lane (acceptance): "Go map iteration order … must not depend on it" is a parent Risk. No story's acceptance checks that the original agrees with itself across two runs.
- The original's driver is not named. `drivers/{go,rust}` suggests the Go driver serves both the original and the Go output, but no body says so.

```findings
- file: .engineering/planning/story/rust-writer.md
  line: 27
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "orders contains the foreign goroutine (Audit, orders.go:132), so refusing it (acceptance 3) and writing a building out/rust/ (acceptance 1) cannot both hold; the body must say which, either narrowing the epic's 'Go -> cir/1 -> Rust without a hand edit' openly or dropping the 'unless a Rust replacement is recorded' clause that reaches into what the epic excludes (epic lines 14, 40-41)"
- file: .engineering/planning/story/rust-writer.md
  line: 18
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome requires the generated crate to pass clippy -D warnings, a quality bar the epic does not ask for; drop it or trace it to a parent sentence"
```
