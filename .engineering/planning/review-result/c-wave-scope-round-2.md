---
format: aep.planning-md/3
id: review-result:c-wave-scope-round-2
kind: review-result
status: active
title: Scope critic, C wave, round 2
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
approve

Every round-1 finding is fixed. I found no gap, no reach beyond the parent and no double claim in the revised set.

- **Matrix row:** `story:c-reader-clang` and `story:c-writer` each claim their own support-matrix row. `story:c-writer` also claims `c` in `LANGUAGES`.
- **Crates and build wiring:** each C story names its own crate, and the shared-surface lists cover `Cargo.toml`, `check.yml` and the `Taskfile.yml` `adapters` task.
- **Overlap with `story:defect-patterns`:** `story:builtin-defect-checks` now says `dropped-error` and `lock-without-unlock` are Rust checks. It says `story:defect-patterns` is done when its pattern files match the same defects, so the two stories do not claim the same deliverable.
- **Across-languages narrowing:** `story:use-cases-page` use 3 now says the checks "read the same `cir/1` every reader produces while being tested on Go only". That records the narrowing.
- **Neighbours:** the section names only capslock, and its Out of Scope excludes other neighbours.
- **Operator's capslock instruction:** the instruction to use capslock as the test repository is claimed once, in `story:builtin-defect-checks` Acceptance. The operator's one-wave request is carried by each story's "Shared surface and merge order" section.

**What you read:** 7 artifacts in full, plus `story:defect-patterns` and `epic:whole-repo-to-syscalls`. I read `epic:c-reader-writer` and `epic:codebase-transformations` first and listed their promises before opening any story. From `epic:c-reader-writer` I extracted 16 promises and traced all 16. The commands were `aep plan artifact show` on each artifact, plus `aep plan artifact graph`, `kinds` and `relations`. I also read `.github/workflows/release-build.yml` and `scripts/read-repo.sh`.

**What you could not establish:**
- `story:c-writer` adds the two binaries to the release archive, which `epic:c-reader-writer` does not mention. I treated it as the means by which an installed `codespec write --lang c` exists, and made no finding.
- `story:builtin-defect-checks` sits outside the pattern mechanism that `epic:codebase-transformations` frames defects under. The story's Out of Scope and its dedup paragraph say so. I carried over round 1's judgement that "find defects" in the epic's Why Now covers it.
- Out of my lane, not in the verdict: the `use-cases-page` acceptance about 0.6.1 output and a clone at tag `0.6.1` (acceptance), and whether `codespec check` is the right new command (design).
- Out of my lane, not in the verdict: `release-build.yml` is edited by `story:c-writer` but is missing from every story's shared-surface list. That belongs to parallel-safety.

```findings
[]
```
