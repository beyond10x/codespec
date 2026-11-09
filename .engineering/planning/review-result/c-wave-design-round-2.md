---
format: aep.planning-md/3
id: review-result:c-wave-design-round-2
kind: review-result
status: active
title: Design critic, C wave, round 2
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
needs-revision

- story:c-reader-clang — its acceptance reads `codespec-run-c` through two lines, and no edge or Done When move covers them: line 41 adds the C reader row to `REPORTS`, and line 42 requires `task check` green. Every `REPORTS` entry is re-run by `docs-check`, which `task check` includes, as `codespec conform --role reader --lang c`. That run needs `codespec-run-c`, which only `story:c-writer` creates. In the reader's own branch the runner is absent, so both lines fail until the writer merges. Round 1's conform read was moved to the epic's Done When, but this same read stayed in the story. The fix, given the one-wave constraint, is to take the reader `REPORTS` row out of the reader story and add it, with `reader-c.json`, to the epic's Done When for the coordinator to do after both merge. The epic's line 60 ("Each adds its own row") then changes too. The alternative is `depends_on story:c-writer`. — .engineering/planning/story/c-reader-clang.md:41-42, crates/docs/src/main.rs:364-373, crates/conformance/src/conform.rs:9-12, Taskfile.yml:47-51

What I read: 5 artifacts, plus `review-result:c-wave-design-round-1`. I ran `aep plan artifact show` on each, `relations`, `graph` and `validate`, which reported valid. I also read `Taskfile.yml`, `crates/docs/src/main.rs` and `crates/conformance/src/conform.rs` to follow the acceptance reads. I walked the 5 ids' declared edges, which are 3 `decomposes` and 2 `informed_by`, and found no cycle. I did not walk the rest of the 150-artifact graph. I traced 6 acceptance reads to producers in the set, and 0 of them have a `depends_on` edge. Five of the six are handled without one: 3 by cutting the read, 2 by moving it to the epic's Done When. The sixth is the finding above. Round 1's use-cases-page finding is fixed: section 3 is "planned" and uses no `codespec check` output. Round 1's conform-run finding is fixed for line 43 only, and the finding above is its remainder.

What I could not establish:
- Whether `scripts/read-repo.sh --deps` exists. That is out of my lane (acceptance or scope) and does not set my verdict.
- Whether the `.github/workflows/check.yml` and `Taskfile.yml` appends are safe to run in parallel. That belongs to parallel-safety.
- Whether adding `c` to `LANGUAGES` in `story:c-writer` alone leaves the support-matrix generation green with no reader row. I read `support::document` only through its callers.

```findings
- file: .engineering/planning/story/c-reader-clang.md
  line: 41
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance adds a C reader row to REPORTS and requires task check green, and docs-check re-runs codespec conform --role reader --lang c, which needs the codespec-run-c that story:c-writer creates; move the reader REPORTS row and reader-c.json to the epic's Done When for the coordinator after both merge, or add depends_on story:c-writer"
```
