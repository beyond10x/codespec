---
format: aep.planning-md/3
id: review-result:c-wave-design-round-1
kind: review-result
status: active
title: Design critic, C wave, round 1
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
needs-revision

- story:c-reader-clang — its conform acceptance runs `codespec conform --lang c --role reader`, which calls `codespec-run-c` (the runner the adapter supplies). `story:c-writer` creates that runner. The body says "After `story:c-writer` merges" in prose, but no `depends_on` edge records it. Without the edge, `aep plan artifact waves` can put the two side by side and the reader's last claimed acceptance can't be checked. Add `depends_on story:c-writer`. — .engineering/planning/story/c-reader-clang.md:36, crates/conformance/src/conform.rs:13 (reader role needs `codespec-run-<lang>`), .engineering/planning/story/c-writer.md Outcome ("`codespec-run-c --build <dir> …`"), `aep plan artifact graph` (only `decomposes` edges on both stories)
- story:use-cases-page — section 3 needs real `codespec check` output on google/capslock, and `story:builtin-defect-checks` creates that command. The acceptance runs every command with the installed 0.6.1, which has no `codespec check`. The only edge is `informed_by`, which means "shaped by", not "needs first". The story therefore ends in one of two states. Either it ships "planned" while the check story merges, or it waits for the check story and a release, and no item flips it. Either add `depends_on story:builtin-defect-checks` and say the page is run against a release containing it, or cut section 3 down to the "planned" text in this story. — .engineering/planning/story/use-cases-page.md:26-28, .engineering/planning/story/use-cases-page.md:38, Cargo.toml:6 (version = "0.6.1")

What I read: 5 artifacts, plus `story:defect-patterns` for its edges. I ran `aep plan artifact show` on each id, plus `relations`, `graph` and `validate`. I walked the declared edges of the 5 ids: 5 edges, 3 of them leaving the set (`epic:codebase-transformations`, `story:defect-patterns`, and the epic parent). I read the whole 489-edge graph for these ids and found no cycle. I traced 3 acceptance reads to producers in the set. They are the `codespec-run-c` read, the `codespec check` output read for the use-cases page, and the `c/case.c` corpus read, which is the reader's own output. Of the first two, 0 have a `depends_on` edge. `validate` reports valid.

What I could not establish:
- Whether `reader_corpus`/`CORPUS_LANGS` need the writer, beyond the runner dependency. I saw only the first lines of `conform.rs`.
- Out of my lane, so not setting the verdict: the use-cases-page acceptance "No page names the third-party library of the measured runs" sits beside section 3 naming google/capslock. That is `plan-critic-acceptance` territory.
- Out of my lane: `scripts/read-repo.sh --deps` is cited by `story:builtin-defect-checks` and I did not check that it exists.
- Not defects: the two C stories do not form a cycle. The runner inside `story:c-writer` is a legitimate half of the writer, and I did not call that a split.

```findings
- file: .engineering/planning/story/c-reader-clang.md
  line: 36
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its conform acceptance runs codespec conform --role reader, which calls codespec-run-c that story:c-writer creates, and no depends_on edge runs from story:c-reader-clang to story:c-writer; add depends_on story:c-writer"
- file: .engineering/planning/story/use-cases-page.md
  line: 26
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "section 3 and the run-every-command-on-0.6.1 acceptance read codespec check output that story:builtin-defect-checks creates, and the only edge is informed_by, so the page has two end states and nothing flips it; add depends_on story:builtin-defect-checks (with the page run against a release containing it) or cut section 3 to the planned text"
```
