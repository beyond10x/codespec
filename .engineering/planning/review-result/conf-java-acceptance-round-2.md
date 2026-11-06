---
format: aep.planning-md/3
id: review-result:conf-java-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2 (adapter-conformance and java-reader)
relations:
- reviews: story:feature-vocabulary
- reviews: story:conformance-cases
- reviews: story:runners-and-manifests
- reviews: story:codespec-conform
- reviews: story:conformance-kit-release
- reviews: story:support-matrix
- reviews: story:java-reader-javac
- reviews: story:java-example-ledger
revision: 1
---
needs-revision
story:conformance-kit-release — the smoke-test bullet runs `codespec conform --lang go --role writer --kit &lt;unpacked&gt;` with exit 0, but the design's signature requires `--out &lt;report.json&gt;` (and the sibling story's Outcome omits it), so the command as written cannot be observed to exit 0, and the guide bullet's `--out work/go-writer.json` contradicts it — .engineering/planning/story/conformance-kit-release.md:35 (design: docs/conformance.md:104)

Round 1 findings, each checked against the revised bodies:

- Fixed: `story:feature-vocabulary` now names the `"const"` values of the `k` tag, and the 49 `k` consts in `schemas/cir-1.schema.json` match 16+17+12+4.
- Fixed: `story:conformance-cases` gives every `sem.*` id a test and its edges.
- Fixed: `story:runners-and-manifests` limits refusals to unclaimed subjects.
- Fixed: `story:codespec-conform` has one stub-writer test per boundary outcome.
- Fixed, with the `--out` issue above left: `story:conformance-kit-release` puts `task conformance-kit` in `task check` and names the guide's command and `summary` line.
- Fixed: `story:support-matrix` has the five-state fixture test and a reader-only language whose write column is `none`.
- Fixed: `story:java-example-ledger` names the four errors, a count of at least 50 over seeds 0–199, and the output strings. The strings match `crates/harness/src/lib.rs:588`.

What I read: 11 of 11 ids (`epic:adapter-conformance`, six stories, `epic:java-reader`, two stories, and `review-result:conf-java-acceptance-round-1`). I ran `aep plan artifact show &lt;id&gt;` for each, read `docs/conformance.md` §§ 6–7, and checked `schemas/cir-1.schema.json` with `jq` and the harness summary format with `grep`.

What I could not establish: I did not count the 7 facts, since they are not `k` consts, and I did not run any task, `codespec`, the docs generator or the site build. Out of my lane: the `--corpus` flag in `story:codespec-conform` is not in `docs/conformance.md` § 6, which is a design-coupling question.

```findings
- file: .engineering/planning/story/conformance-kit-release.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the smoke-test bullet runs `codespec conform --lang go --role writer --kit &lt;unpacked&gt;` with exit 0, but the design's signature requires `--out &lt;report.json&gt;` (and story:codespec-conform's Outcome omits it), so the command as written cannot be observed to exit 0 and the guide bullet's `--out work/go-writer.json` contradicts it"
```
