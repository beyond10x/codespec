---
format: aep.planning-md/3
id: review-result:c-wave-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, C wave, round 1
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
needs-revision

- story:c-writer — the acceptance names `assign-add` as an overflow case, but its only call is `AddTo(2, 3)`, which never overflows, so a writer emitting a plain signed `+` passes under `-fsanitize=undefined`. The case that overflows is `sem-int-wrap` (calls `Add(9223372036854775807, 1)`, edge `MaxInt64 + 1`), and the "(or the kit's overflow case)" hedge should become that name — .engineering/planning/story/c-writer.md:33
- story:builtin-defect-checks — "No fixture, test or doc names the library the checks came from" gives no name, list or command to check against, so nobody but the drafter can tell whether it holds. It needs a named deny-list or a pointer to where the name is recorded, plus a grep that fails on a hit — .engineering/planning/story/builtin-defect-checks.md:45
- story:use-cases-page — "No page names the third-party library of the measured runs" has the same gap, and "no page" does not say whether it means this page or the whole site. It needs a name or deny-list to grep and a scope — .engineering/planning/story/use-cases-page.md:42
- story:use-cases-page — "read on the day the page is written" and "A claim that could not be checked is not written" cannot be verified afterwards, since only the author knows what was read and what was dropped. The checkable part is a link per claim. The acceptance should require that each neighbour claim carries a link and a quoted or cited passage from that page — .engineering/planning/story/use-cases-page.md:40-41

What I read: 5 of 5 ids, with `aep plan artifact show <id>` for each, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I also checked the symbols and paths the acceptances name against the tree with `git grep`, `ls`, `sed` and `cat`.

What I could not establish:
- Whether `epic:c-reader-writer`'s "Done When" (one sentence joining four outcomes) should be split. I left it, since an epic's done condition aggregates its stories.
- Out of my lane (scope): the epic's "the support matrix lists C" is claimed by neither story's acceptance, so no story delivers it.
- Out of my lane (scope): `story:builtin-defect-checks` states exit codes 0/1/2 and the `codespec-check-report/1` JSON in its outcome, but no acceptance line tests them.
- Out of my lane (design): `story:builtin-defect-checks` decomposes `epic:codebase-transformations`, and I did not judge that relation.

```findings
- file: .engineering/planning/story/c-writer.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `assign-add` as the overflow case, but its only call is AddTo(2, 3), which never overflows, so a plain signed `+` passes under -fsanitize=undefined; the overflowing kit case is `sem-int-wrap` and the acceptance should name it"
- file: .engineering/planning/story/builtin-defect-checks.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"No fixture, test or doc names the library the checks came from\" names no string, deny-list or command, so it cannot be checked by anyone but the drafter"
- file: .engineering/planning/story/use-cases-page.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"No page names the third-party library of the measured runs\" names no string or deny-list to check and does not say whether \"no page\" means this page or the whole site"
- file: .engineering/planning/story/use-cases-page.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"read on the day the page is written\" and \"A claim that could not be checked is not written\" cannot be verified after the fact; the acceptance should require a link plus a cited or quoted passage per neighbour claim"
```
