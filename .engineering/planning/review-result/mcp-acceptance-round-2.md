---
format: aep.planning-md/3
id: review-result:mcp-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic on the MCP epic, round 2
relations:
- reviews: story:mcp-server-read-tools
- reviews: story:mcp-semantic-tools
- reviews: story:mcp-rewrite-preview
- reviews: story:mcp-apply-in-place
- reviews: story:mcp-verify-change
- reviews: story:reader-function-spans
revision: 1
---
needs-revision
story:reader-function-spans — the span rule is "end with its closing brace" for "every function", yet the Outcome records "one record per function declaration" and body-less declarations (Go external functions, C prototypes) have no closing brace, so the check cannot be applied to them and the acceptance does not say what their span is or whether they get a record — .engineering/planning/story/reader-function-spans.md:38
story:reader-function-spans — the edge-case bullet says only that two functions on one line, attributes or annotations, CRLF and a byte-order mark "each have a case", naming no expected span for any of them (whether attributes are inside the span, whether the BOM's three bytes count in offsets), so a case can pass with either answer — .engineering/planning/story/reader-function-spans.md:40
story:mcp-apply-in-place — the Outcome says "every other byte of the file is kept" while the acceptance requires that after `apply` the file "imports" `strings`, an edit outside every function span, so the two cannot both be checked and the body does not say the import line is the exception — .engineering/planning/story/mcp-apply-in-place.md:27

**What I read:** 6 of 6 ids (the five MCP stories and `story:reader-function-spans`), using `aep plan artifact show <id>` for each. I also read `story:cir-rewrite` and `story:eval-conformance-runner`, the parent epic, and the tree for `codespec-calls/1`, `Effects`, and body-less functions.

**What I could not establish:**
- Whether `examples/series` and `examples/inventory` hold the cases the acceptances rely on (a `foreign` statement, an unresolved call). That is a fixture question and I did not open them.
- Round 1 findings were not shown to me, so I judged only the set as it stands now. `origin` is therefore `introduced` for all three findings, which is a judgement and not a comparison against round 1.
- Out of my lane: none seen.

```findings
- file: .engineering/planning/story/reader-function-spans.md
  line: 38
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the span rule is 'end with its closing brace' for every function, yet the Outcome records one record per function declaration and body-less declarations (Go external functions, C prototypes) have no closing brace, so the check cannot be applied to them and the acceptance does not say what their span is or whether they get a record"
- file: .engineering/planning/story/reader-function-spans.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the edge-case bullet says only that two functions on one line, attributes or annotations, CRLF and a byte-order mark each have a case, naming no expected span for any of them (whether attributes are inside the span, whether the BOM's three bytes count in offsets), so a case can pass with either answer"
- file: .engineering/planning/story/mcp-apply-in-place.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says every other byte of the file is kept while the acceptance requires that after apply the file imports strings, an edit outside every function span, so the two cannot both be checked and the body does not say the import line is the exception"
```
