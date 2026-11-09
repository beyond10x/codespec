---
format: aep.planning-md/3
id: review-result:c-wave-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, C wave, round 1
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
needs-revision

Findings:

story:c-reader-clang — its acceptance reads `codespec-run-c`, which story:c-writer creates, and the body carries only a `decomposes` edge to the epic, so one wave would hold both and the reader's conformance run cannot be checked until the writer lands. Add `depends_on story:c-writer`, or merge the two. — .engineering/planning/story/c-reader-clang.md:36, .engineering/planning/story/c-writer.md:24

story:use-cases-page — section 3 and the status-page link read the output and shipped state of story:builtin-defect-checks, and the only edge is `informed_by` (line 8), so one wave would hold both and the page would be written as "planned" against a story landing beside it. Add `depends_on story:builtin-defect-checks`, or merge the two. — .engineering/planning/story/use-cases-page.md:26-29, .engineering/planning/story/builtin-defect-checks.md:9

story:c-writer — this and story:c-reader-clang both edit `crates/docs/src/main.rs` (`REPORTS`, line 235), `crates/docs/src/support.rs` (`LANGUAGES`, line 44), the `adapters` task and manifest loops in `Taskfile.yml` (lines 129-130, 183-184), the test matrix in `.github/workflows/check.yml` (lines 39-40) and the generated `website/data/support-matrix.json`. Neither body says so. This surface is inferred from the tree, plus the epic's "both adapters are Rust crates in the workspace" (.engineering/planning/epic/c-reader-writer.md). Remedies: an ordering edge that records these files as its reason, or splitting the surface so the two no longer share it. — .engineering/planning/story/c-writer.md:32, .engineering/planning/story/c-reader-clang.md:38

story:builtin-defect-checks — it creates the Rust crate `crates/check`, and each C adapter crate is also a new workspace member, so all three append to the single-line `members = [...]` array in `Cargo.toml` and to `Cargo.lock`. The two C stories and this one also each add a line to the test matrix in `.github/workflows/check.yml`. None of the bodies mentions this. The members and `Cargo.lock` surface is inferred, since only `crates/check/tests/fixtures/` is cited. This collision is not a file-merge problem that parallelism fixes. Remedies: an ordering edge that records `Cargo.toml` as its reason, or splitting the surface so the three stop sharing it. — Cargo.toml:3, .engineering/planning/story/builtin-defect-checks.md:40

Read: 4 artifacts, plus epic:c-reader-writer and story:defect-patterns for context. Commands run: `aep plan artifact show` on each of the four, `aep plan artifact graph`, `relations`, `waves`, `git grep`, and a read of `crates/docs`, `crates/conformance`, `Taskfile.yml` and `.github/workflows`.

Surfaces established: cited 4 (partially, each by files its body names), inferred 0 as a whole item. The collision surfaces in findings 3 and 4 are inferred. Unplaceable 0. `aep plan artifact waves` itself lists all four as `unassessed`, so the store records no `scope` for any of them.

Acceptance reads traced to a producer in the set: 2 (c-reader-clang to c-writer's runner, use-cases-page to builtin-defect-checks). `depends_on` edges recording them: 0.

Could not establish:
- Neither C body names the directory its crate lives in (for example `adapters/c-read`), so the exact new paths are inferred.
- Whether `crates/cli/src/lib.rs` (the `Cmd` enum) is also edited by an item outside this set is not visible to me.
- Out of lane: use-cases-page requires every command to be run with the installed 0.6.1 `codespec`, which cannot include `codespec check`, so section 3 can only ever say "planned". That is an acceptance or scope matter.

```findings
- file: .engineering/planning/story/c-reader-clang.md
  line: 36
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads codespec-run-c, which story:c-writer creates, and no depends_on edge runs from this story to story:c-writer (only decomposes), so one wave would hold both; add the depends_on edge or merge the two"
- file: .engineering/planning/story/use-cases-page.md
  line: 26
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "section 3 and the status-page link read the output and shipped state of story:builtin-defect-checks, and only an informed_by edge (line 8) links them, so one wave would hold both; add depends_on story:builtin-defect-checks or merge the two"
- file: .engineering/planning/story/c-writer.md
  line: 32
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this and story:c-reader-clang both edit crates/docs/src/main.rs REPORTS, crates/docs/src/support.rs LANGUAGES, the Taskfile.yml adapters task and manifest loops, the check.yml test matrix and the generated website/data/support-matrix.json, and neither body says so (surface inferred from the tree and the epic); add an ordering edge recording the shared files or split the surface"
- file: .engineering/planning/story/builtin-defect-checks.md
  line: 40
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it creates crates/check, and each C adapter crate is also a new workspace member, so all append to the single-line members array in Cargo.toml and to Cargo.lock and add a check.yml matrix line, and no body says so (surface inferred; only crates/check/tests/fixtures is cited); add an ordering edge recording Cargo.toml or split the surface"
```
