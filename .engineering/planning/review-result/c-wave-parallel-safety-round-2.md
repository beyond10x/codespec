---
format: aep.planning-md/3
id: review-result:c-wave-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, C wave, round 2
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

story:c-writer — the shared-surface section omits `website/docs/reference/crates.md`, which `codespec-docs` regenerates from `cargo metadata`. All three crate-adding stories change it (four new packages), so it collides (inferred from `crates/docs/src/crates.rs`). The page holds a package count and one row per crate, so "keeping both entries" cannot merge it. Add the file and say generated files (`crates.md`, `support-matrix.json`) are regenerated after each merge, or split the surface. The same section is copied into the other three bodies. — .engineering/planning/story/c-writer.md:57-68, crates/docs/src/crates.rs:129

story:c-reader-clang — it and story:c-writer both need the single `c` entry in `crates/docs/src/support.rs` `LANGUAGES` (cited: the reader's matrix row at line 41, the writer's line 42 at c-writer.md). The section calls each "appending one entry", so keeping both entries gives a duplicate `c` column. The body does not say which story adds it. Name one owner and have the other rely on it, which the merge order makes the writer, or order them on that file. — .engineering/planning/story/c-reader-clang.md:41,62

What I read: 4 artifacts, plus `review-result:c-wave-parallel-safety-round-1`. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `relations` and `waves`, and `git grep` and reads of `Cargo.toml`, `Taskfile.yml`, `.github/workflows/`, `crates/docs/src/{main,support,crates}.rs` and `crates/conformance/src/conform.rs`.

Round 1 status:
- **Finding 3 (docs, Taskfile and CI surface shared by the two C stories):** fixed. The section names the files and the merge order.
- **Finding 4 (`Cargo.toml` and `Cargo.lock` shared by all three crate-adding stories):** fixed. The section names the files and the merge order.
- **Finding 1 (reader reads the runner the writer creates):** the reader's acceptance now has the conformance run as a coordinator step after `story:c-writer`. The other reader bullets need only the Go tools. I count it fixed. If you want the edge in the store, it is still missing: `depends_on` edges for these four stories number 0.
- **Finding 2 (use-cases-page reads the checks story's output):** `story:use-cases-page` now writes section 3 as "planned" and shows no `check` output. It no longer reads that story's state. I count it fixed.

Surfaces: 4 items cited, 0 inferred, 0 unplaceable. `aep plan artifact waves` still lists all four as `unassessed` because the store has no `scope` for them. Of the two collisions I report, the `crates.md` one rests on an inferred surface and the `LANGUAGES` one is cited. Acceptance reads traced to a producer in the set: 2 (the reader's last bullet reads the writer's runner, and the use-cases page reads the checks story's plan). `depends_on` edges recording them: 0, both accepted above.

Could not establish:
- The shared lists describe the `adapters` task as "one entry" per story. In the tree, `Taskfile.yml:129-130` are single `cargo build -p …` and `cp` lines, and `Cargo.toml:3` is a one-line array. The two C stories edit the same line there, so the merge is a union inside a line, not an append. I did not make this a finding because the coordinator already resolves it by hand.
- `ux/baseline.json` is updated by `story:use-cases-page`. Whether the generated pages the other three stories change (`cli.md`, `crates.md`, the support page) move the ux values is not visible to me.
- Whether `website/docs/status.mdx` or `status.json` changes with `story:builtin-defect-checks` is not visible to me.
- Out of lane: the reader's last Acceptance bullet cannot be checked inside its own story (acceptance critic).

```findings
- file: .engineering/planning/story/c-writer.md
  line: 57
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the shared-surface section omits website/docs/reference/crates.md, regenerated from cargo metadata with a package count and a row per new crate by story:c-reader-clang, story:c-writer and story:builtin-defect-checks (surface inferred from crates/docs/src/crates.rs), so keeping both entries cannot merge it; list it and say generated files are regenerated after each merge, or split the surface"
- file: .engineering/planning/story/c-reader-clang.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story and story:c-writer both need the one c entry in crates/docs/src/support.rs LANGUAGES, yet the section calls each an appended entry, so keeping both yields a duplicate column; name one owning story and have the other rely on it, or order them on that file"
```
