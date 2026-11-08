---
format: aep.planning-md/3
id: review-result:externs-design-round-1
kind: review-result
status: active
title: Design critic, round 1
relations:
- reviews: story:cir1-externs
- reviews: story:go-extern-calls
- reviews: story:effects-report
- reviews: story:effects-example
revision: 1
---
needs-revision

Findings:

story:cir1-externs — the set cuts one feature in two. Landing this story alone leaves `task check` red: `conformance/kit.json` is generated from the vocabulary this story extends, and the Go reader and writer claim `expr.*`. The tree turns green only when `story:go-extern-calls` adds the case and `kit.json`. Either move the `expr-extern` case and `conformance/kit.json` into this story, or have this story's scope keep the Go manifests from claiming `expr.extern` until the other lands. The outcome sentence "Every adapter in this repository either handles the new expression or says it does not" is false for Go until then. — .engineering/planning/story/cir1-externs.md:16, .engineering/planning/story/cir1-externs.md:25, .engineering/planning/story/go-extern-calls.md:49

story:effects-report — its summary scope names "`strings.*` functions used by examples and kit cases", but those functions come from `examples/envcheck`, which `story:effects-example` creates later, and the kit case from `story:go-extern-calls`. The summary file is not in `story:effects-example`'s scope. If `envcheck` calls a `strings` function the summary lacks, the report gets an `unknown` where the example expects `[]`. List the `strings` and `strconv` functions by name in this body, or add `summaries/go-std.effects.json` to the scope of `story:effects-example`. — .engineering/planning/story/effects-report.md:34, .engineering/planning/story/effects-example.md:39

What I read: 5 artifacts (the epic and four stories), through `aep plan artifact show` for each, plus `relations`, `graph` and `validate`. I also read `docs/static-analysis.md`, the `Taskfile.yml` gate (`check`, `conformance-cases`, `conformance-kit`, `docs-check`) and the verdict table in `crates/conformance/src/conform.rs:90-98`.

What I could not establish:
- Cycle and chain: none. I walked the 8 declared edges among the set, 4 `decomposes` and 4 `depends_on`, and the full graph of 245 edges. The graph is a diamond: `cir1-externs`, then `go-extern-calls` and `effects-report` in parallel, then `effects-example`. `validate` reports `valid`.
- Acceptance reads of state a sibling produces: 6 traced, all 6 recorded by an edge.
- Not run: whether `task docs-check` rejects a claimed feature with no case. From the verdict table, a claimed feature with no case is `fail, no_case`. `conformance-kit` runs `codespec conform` on the Go writer and requires exit 0.
- Out of my lane, for the acceptance critic: `story:go-extern-calls` and the epic say the Rust writer's `expr.extern` is "not claimed and `pass`". The table at `crates/conformance/src/conform.rs:97` makes an unclaimed feature whose case is refused `refused`, not `pass`.

```findings
- file: .engineering/planning/story/cir1-externs.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the set cuts one feature in two: landing this story alone leaves task check red, because conformance/kit.json is generated from the vocabulary this story extends and the Go reader and writer claim expr.* with no expr-extern case until story:go-extern-calls adds the case and kit.json, so the case and kit.json belong here or the Go manifests must not claim expr.extern until that story lands, and the outcome sentence 'Every adapter in this repository either handles the new expression or says it does not' is false for Go until then"
- file: .engineering/planning/story/effects-report.md
  line: 34
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the summary scope names 'strings.* functions used by examples and kit cases', which come from examples/envcheck created later by story:effects-example and from the kit case in story:go-extern-calls, and the summary file is not in story:effects-example's scope, so the body must list the strings and strconv functions by name or story:effects-example must take the summary file into its scope"
```
