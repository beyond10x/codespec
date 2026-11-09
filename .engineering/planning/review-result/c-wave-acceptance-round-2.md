---
format: aep.planning-md/3
id: review-result:c-wave-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, C wave, round 2
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
approve

What I read: 5 of 5 ids (`epic:c-reader-writer`, `story:c-reader-clang`, `story:c-writer`, `story:builtin-defect-checks`, `story:use-cases-page`). I ran `aep plan artifact show` on each, plus `aep plan artifact show review-result:c-wave-acceptance-round-1`, `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I checked the named symbols and paths against the tree with `git grep` and `ls`:
- The kit cases `sem-int-wrap`, `expr-binop`, `stmt-for`, `stmt-switch`, `op-add`.
- `CORPUS_LANGS`, `REPORTS`, `LANGUAGES`, `scripts/read-repo.sh --deps`, `--describe`.
- The tag `0.6.1`, the `website/` files, and the `ux-check` and `docs-generate` tasks.
- The runner contract (exit codes 0, 2, 1) in `crates/conformance/src/conform.rs`.

Round 1 findings, all four resolved:
- **`story:c-writer`:** the acceptance now names `sem-int-wrap` (`Add(9223372036854775807, 1)`). It requires a plain signed `+` copy built under `-fsanitize=undefined -fno-sanitize-recover` to fail the case. The case's `calls.json` has that call.
- **`story:builtin-defect-checks`:** the deny-list is now a command the coordinator runs, `git diff origin/main --name-only | xargs grep -i -F -f <deny-list>`, which must print nothing. The scope is every changed file.
- **`story:use-cases-page`, deny-list:** the same gap is now closed by a `grep -r -i -F -f <deny-list>` over `website/docs`, `website/src` and `website/product.json`. That scope is explicit.
- **`story:use-cases-page`, neighbour claims:** each capslock claim must link a page of capslock's own documentation and quote the passage it rests on. That is checkable afterwards.

What I could not establish:
- The deny-list contents, which are held outside the repository by design. I judged the check by its command form only.
- Out of my lane (design): `story:builtin-defect-checks` still decomposes `epic:codebase-transformations`. I did not judge that relation.
- Out of my lane (scope): the checks' exit codes 0, 1 and 2 are tested by an acceptance bullet in this revision. I did not re-check the epic's "support matrix lists C" coverage.

```findings
[]
```
